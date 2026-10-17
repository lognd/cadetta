// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
// frob:ticket 01M54BDDBT01RBBDS3AQ8ZNA4K
// frob:ticket 01M54EDNCXH7QFEEGJQNAE1M1N
using System;
using System.Runtime.InteropServices;
using Cadetta.SolidWorks.AddIn.Client;
using Cadetta.SolidWorks.AddIn.Commands;
using Cadetta.SolidWorks.AddIn.Logging;
using Cadetta.SolidWorks.AddIn.Pane;
using Cadetta.SolidWorks.Com.Busy;
using Cadetta.SolidWorks.Com.Push;
using Cadetta.SolidWorks.Commands;
using Cadetta.SolidWorks.Logging;
using Microsoft.Win32;
using SolidWorks.Interop.sldworks;
using SolidWorks.Interop.swconst;
using SolidWorks.Interop.swpublished;

namespace Cadetta.SolidWorks.AddIn
{
    /// <summary>The SOLIDWORKS COM add-in: hosts the task pane and the Cadetta menu, keeps them in step with the active document and selection, and runs the link, sync and why commands.</summary>
    [ComVisible(true)]
    [Guid(AddInGuid)]
    [ProgId("Cadetta.SolidWorks.AddIn")]
    [ClassInterface(ClassInterfaceType.AutoDispatch)]
    public sealed class CadettaAddIn : ISwAddin
    {
        /// <summary>CLSID of the add-in.</summary>
        public const string AddInGuid = "b3a7d1c0-52e4-4f3d-9a61-2c8e7f0d4b15";

        private ISldWorks? _sw;
        private ITaskpaneView? _pane;
        private TaskPaneControl? _control;
        private PaneModel? _model;
        private CommandActions? _actions;
        private int _cookie;
        private Log _log = Log.Null();

        private const string MenuName = "&Cadetta";

        /// <inheritdoc />
        public bool ConnectToSW(object thisSw, int cookie)
        {
            _log = AddInLog.OpenDefault();
            _log.Warn("add-in: connect, cookie " + cookie);
            AddInDependencyResolver.Install(_log);
            try
            {
                return Connect(thisSw, cookie);
            }
            catch (Exception e)
            {
                // SOLIDWORKS swallows exceptions from ConnectToSW; the log is the only trace of a failed load.
                _log.Error("add-in: connect failed: " + e);
                return false;
            }
        }

        private bool Connect(object thisSw, int cookie)
        {
            _sw = (ISldWorks)thisSw;
            _sw.SetAddinCallbackInfo2(0, this, cookie);
            var cli = new CadettaCli(new ProcessCommandRunner(_log), AddInSettings.Executable(), _log);
            _model = new PaneModel(cli, _log);
            _actions = new CommandActions(new CadettaCommands(new ProcessCommandRunner(_log), AddInSettings.Executable(), _log), _log, cli);
            _cookie = cookie;
            _pane = _sw.CreateTaskpaneView2("", "Cadetta");
            _control = (TaskPaneControl)_pane.AddControl(TaskPaneControl.ProgIdText, "");
            _control.CommandRequested += RunVerb;
            AddMenu();
            var app = (SldWorks)_sw;
            app.ActiveModelDocChangeNotify += OnActiveDocChange;
            app.FileOpenPostNotify += OnFileOpen;
            Refresh();
            return true;
        }

        /// <inheritdoc />
        public bool DisconnectFromSW()
        {
            _log.Info("add-in: disconnect");
            UnhookSelection();
            if (_sw != null)
            {
                var app = (SldWorks)_sw;
                app.ActiveModelDocChangeNotify -= OnActiveDocChange;
                app.FileOpenPostNotify -= OnFileOpen;
            }
            if (_control != null)
            {
                _control.CommandRequested -= RunVerb;
            }
            RemoveMenu();
            _pane?.DeleteView();
            _pane = null;
            _control = null;
            _sw = null;
            return true;
        }

        private static readonly string[][] MenuItems =
        {
            new[] { "Link", "Link dimension to selected path", "MenuLink" },
            new[] { "Unlink", "Unlink dimension", "MenuUnlink" },
            new[] { "Preview", "Sync preview (dry run)", "MenuPreview" },
            new[] { "Sync", "Sync", "MenuSync" },
            new[] { "Why", "Why this path", "MenuWhy" },
            new[] { "AssignArray", "Assign array to selected instances", "MenuAssignArray" },
        };

        private static string MenuText(string[] item) => item[1] + "@" + MenuName;

        private void AddMenu()
        {
            for (var i = 0; i < MenuItems.Length; i++)
            {
                var item = MenuItems[i];
                var id = _sw!.AddMenuItem5((int)swDocumentTypes_e.swDocPART, _cookie, MenuText(item), i, item[2], "MenuEnabled", item[1], "");
                _sw.AddMenuItem5((int)swDocumentTypes_e.swDocASSEMBLY, _cookie, MenuText(item), i, item[2], "MenuEnabled", item[1], "");
                _log.Info("add-in: menu item '" + item[1] + "' " + (id >= 0 ? "added as " + id : "REFUSED (" + id + ")"));
            }
        }

        private void RemoveMenu()
        {
            if (_sw == null)
            {
                return;
            }
            foreach (var item in MenuItems)
            {
                _sw.RemoveMenu((int)swDocumentTypes_e.swDocPART, MenuText(item), "");
                _sw.RemoveMenu((int)swDocumentTypes_e.swDocASSEMBLY, MenuText(item), "");
            }
        }

        /// <summary>Menu callback: link the selected dimension to the selected pane path.</summary>
        public void MenuLink() => RunVerb("Link");

        /// <summary>Menu callback: unlink the selected dimension.</summary>
        public void MenuUnlink() => RunVerb("Unlink");

        /// <summary>Menu callback: run `cadetta sync --dry-run`.</summary>
        public void MenuPreview() => RunVerb("Preview");

        /// <summary>Menu callback: run `cadetta sync`.</summary>
        public void MenuSync() => RunVerb("Sync");

        /// <summary>Menu callback: run `cadetta why` for the selected pane path.</summary>
        public void MenuWhy() => RunVerb("Why");

        /// <summary>Menu callback: assign the selected instances the references of the nearest array frames.</summary>
        public void MenuAssignArray() => RunVerb("AssignArray");

        /// <summary>Menu enable callback: 1 (enabled) only while a saved document is active.</summary>
        public int MenuEnabled() => _model?.Project != null ? 1 : 0;

        private void RunVerb(string verb)
        {
            if (_sw == null || _model == null || _control == null || _actions == null)
            {
                return;
            }
            var path = _control.SelectedPath;
            _log.Info("add-in: command " + verb + (path == null ? "" : " on " + path));
            CommandOutcome outcome;
            _cliBusy = true;
            try
            {
                outcome = Execute(verb, path);
            }
            catch (Exception e)
            {
                _log.Error("add-in: command " + verb + " failed: " + e);
                outcome = CommandOutcome.Refused("failed: " + e.Message);
            }
            finally
            {
                _cliBusy = false;
            }
            _log.Info("add-in: command " + verb + (outcome.IsOk ? " ok" : " refused" + (outcome.Code == null ? "" : " " + outcome.Code)) + ": " + outcome.Message);
            Refresh();
            _control.ShowResult((outcome.IsOk ? "" : "Refused: ") + outcome.Message);
        }

        private CommandOutcome Execute(string verb, string? path)
        {
            var project = _model!.Project;
            var adapter = project?.Pick(_model.ChosenAdapterId)?.Id;
            switch (verb)
            {
                case "Sync":
                    return _actions!.Sync(project, adapter, false, new string[0]);
                case "Preview":
                    return _actions!.Sync(project, adapter, true, new string[0]);
                case "Why":
                    return _actions!.Why(project, path);
            }
            var docPath = SwSelection.ActiveDocumentPath(_sw!);
            var model = _sw!.ActiveDoc as IModelDoc2;
            if (docPath == null || model == null)
            {
                return CommandOutcome.Refused("open and save a document first");
            }
            var doc = new SwComPushDocument(model, docPath, _log);
            var probe = new SwComBusyProbe(_sw, _log);
            if (verb == "AssignArray")
            {
                return _actions!.AssignArray(project, doc, probe, SwSelection.SelectedComponents(_sw), new SwComComponentReferences(_sw, _log), path);
            }
            var dimension = SwSelection.SelectedDimensionName(_sw);
            return verb == "Link" ? _actions!.Link(doc, probe, dimension, path) : _actions!.Unlink(doc, probe, dimension);
        }

        private int OnActiveDocChange()
        {
            RefreshLater("active-doc-change");
            return 0;
        }

        private int OnFileOpen(string fileName)
        {
            RefreshLater("file-open " + fileName);
            return 0;
        }

        private bool _refreshQueued;
        private bool _cliBusy;

        /// <summary>Queues one pane refresh on the UI message loop so the slow names call never runs inside a SOLIDWORKS event callback (which crashed SOLIDWORKS on reopen).</summary>
        private void RefreshLater(string source)
        {
            if (_cliBusy)
            {
                // The adapter opens documents in this SOLIDWORKS while the CLI runs; reacting would loop forever.
                _log.Debug("add-in: ignoring " + source + " raised while a cadetta command is running");
                return;
            }
            var control = _control;
            if (control == null || !control.IsHandleCreated)
            {
                Refresh();
                return;
            }
            if (_refreshQueued)
            {
                return;
            }
            _refreshQueued = true;
            _log.Debug("add-in: pane refresh queued by " + source);
            control.BeginInvoke((Action)(() =>
            {
                _refreshQueued = false;
                RefreshIfDocumentChanged();
            }));
        }

        private string? _loadedPath;

        /// <summary>Reloads the pane only when the active document differs from the one last loaded; otherwise just re-renders the selection.</summary>
        private void RefreshIfDocumentChanged()
        {
            if (_sw == null)
            {
                return;
            }
            string? path;
            try
            {
                path = SwSelection.ActiveDocumentPath(_sw);
            }
            catch (Exception e)
            {
                _log.Error("add-in: active document lookup failed: " + e);
                return;
            }
            if (path != null && path == _loadedPath && _model?.Project != null)
            {
                _log.Debug("add-in: pane already loaded for " + path);
                Show();
                return;
            }
            Refresh();
        }

        private void Refresh()
        {
            if (_sw == null || _model == null || _control == null)
            {
                return;
            }
            _cliBusy = true;
            try
            {
                var path = SwSelection.ActiveDocumentPath(_sw);
                _loadedPath = path;
                _log.Debug("add-in: pane refresh for " + (path ?? "(no document)"));
                _model.Load(path);
                HookSelection();
                Show();
            }
            catch (Exception e)
            {
                // A COM event callback swallows exceptions silently, so the pane would just stay empty; log it.
                _log.Error("add-in: pane refresh failed: " + e);
            }
            finally
            {
                _cliBusy = false;
            }
        }

        private void Show()
        {
            if (_sw == null || _model == null || _control == null)
            {
                return;
            }
            _control.Render(_model.Status, _model.View(SwSelection.SelectedDimension(_sw)));
        }

        private ModelDoc2? _hooked;

        private void HookSelection()
        {
            var doc = _sw?.ActiveDoc as ModelDoc2;
            if (ReferenceEquals(doc, _hooked))
            {
                return;
            }
            UnhookSelection();
            _hooked = doc;
            if (doc is PartDoc part)
            {
                part.UserSelectionPostNotify += OnSelection;
                part.DestroyNotify += OnHookedDestroy;
            }
            else if (doc is AssemblyDoc asm)
            {
                asm.UserSelectionPostNotify += OnSelection;
                asm.DestroyNotify += OnHookedDestroy;
            }
        }

        /// <summary>Detaches the selection and destroy handlers from the previously hooked document so a closing document is never left with a live managed handler.</summary>
        private void UnhookSelection()
        {
            var old = _hooked;
            _hooked = null;
            if (old is PartDoc part)
            {
                part.UserSelectionPostNotify -= OnSelection;
                part.DestroyNotify -= OnHookedDestroy;
            }
            else if (old is AssemblyDoc asm)
            {
                asm.UserSelectionPostNotify -= OnSelection;
                asm.DestroyNotify -= OnHookedDestroy;
            }
        }

        private int OnHookedDestroy()
        {
            _log.Debug("add-in: hooked document is closing, unhooking");
            UnhookSelection();
            return 0;
        }

        private int OnSelection()
        {
            Show();
            return 0;
        }

        /// <summary>Registers the add-in with SOLIDWORKS, run by regasm: HKLM Addins key with default 1 (loaded at startup) and the HKCU AddInsStartup flag. frob:ticket ~EXF7N54</summary>
        [ComRegisterFunction]
        public static void Register(Type t)
        {
            using (var key = Registry.LocalMachine.CreateSubKey(AddinsKey))
            {
                // Default 1 = load at startup; 0 leaves the add-in listed but never activated.
                key.SetValue(null, 1);
                key.SetValue("Title", AddInInfo.Title);
                key.SetValue("Description", "Link SOLIDWORKS dimensions to Cadetta paths");
            }
            using (var key = Registry.CurrentUser.CreateSubKey(StartupKey))
            {
                key.SetValue(null, 1);
            }
        }

        /// <summary>Removes the registration written by <see cref="Register"/>. frob:ticket ~EXF7N54</summary>
        [ComUnregisterFunction]
        public static void Unregister(Type t)
        {
            Registry.LocalMachine.DeleteSubKey(AddinsKey, false);
            Registry.CurrentUser.DeleteSubKey(StartupKey, false);
        }

        private const string AddinsKey = @"SOFTWARE\SolidWorks\Addins\{" + AddInGuid + "}";
        private const string StartupKey = @"Software\SolidWorks\AddInsStartup\{" + AddInGuid + "}";
    }
}

// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
using System;
using System.Runtime.InteropServices;
using Cadetta.SolidWorks.AddIn.Client;
using Cadetta.SolidWorks.AddIn.Logging;
using Cadetta.SolidWorks.AddIn.Pane;
using Cadetta.SolidWorks.Logging;
using Microsoft.Win32;
using SolidWorks.Interop.sldworks;
using SolidWorks.Interop.swpublished;

namespace Cadetta.SolidWorks.AddIn
{
    /// <summary>The SOLIDWORKS COM add-in: hosts the task pane and keeps it in step with the active document and selection.</summary>
    [ComVisible(true)]
    [Guid(AddInGuid)]
    [ProgId("Cadetta.SolidWorks.AddIn")]
    [ClassInterface(ClassInterfaceType.None)]
    public sealed class CadettaAddIn : ISwAddin
    {
        /// <summary>CLSID of the add-in.</summary>
        public const string AddInGuid = "b3a7d1c0-52e4-4f3d-9a61-2c8e7f0d4b15";

        private ISldWorks? _sw;
        private ITaskpaneView? _pane;
        private TaskPaneControl? _control;
        private PaneModel? _model;
        private Log _log = Log.Null();

        /// <inheritdoc />
        public bool ConnectToSW(object thisSw, int cookie)
        {
            _log = AddInLog.OpenDefault();
            _log.Info("add-in: connect, cookie " + cookie);
            _sw = (ISldWorks)thisSw;
            _sw.SetAddinCallbackInfo2(0, this, cookie);
            var cli = new CadettaCli(new ProcessCommandRunner(_log), AddInSettings.Executable(), _log);
            _model = new PaneModel(cli, _log);
            _pane = _sw.CreateTaskpaneView2("", "Cadetta");
            _control = (TaskPaneControl)_pane.AddControl(TaskPaneControl.ProgIdText, "");
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
            if (_sw != null)
            {
                var app = (SldWorks)_sw;
                app.ActiveModelDocChangeNotify -= OnActiveDocChange;
                app.FileOpenPostNotify -= OnFileOpen;
            }
            _pane?.DeleteView();
            _pane = null;
            _control = null;
            _sw = null;
            return true;
        }

        private int OnActiveDocChange()
        {
            Refresh();
            return 0;
        }

        private int OnFileOpen(string fileName)
        {
            Refresh();
            return 0;
        }

        private void Refresh()
        {
            if (_sw == null || _model == null || _control == null)
            {
                return;
            }
            var path = SwSelection.ActiveDocumentPath(_sw);
            _model.Load(path);
            HookSelection();
            Show();
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
            _hooked = doc;
            if (doc is PartDoc part)
            {
                part.UserSelectionPostNotify += OnSelection;
            }
            else if (doc is AssemblyDoc asm)
            {
                asm.UserSelectionPostNotify += OnSelection;
            }
        }

        private int OnSelection()
        {
            Show();
            return 0;
        }

        /// <summary>Registers the add-in with SOLIDWORKS (HKLM\SOFTWARE\SolidWorks\Addins), run by regasm.</summary>
        [ComRegisterFunction]
        public static void Register(Type t)
        {
            using (var key = Registry.LocalMachine.CreateSubKey(@"SOFTWARE\SolidWorks\Addins\{" + AddInGuid + "}"))
            {
                key.SetValue(null, 0);
                key.SetValue("Title", AddInInfo.Title);
                key.SetValue("Description", "Link SOLIDWORKS dimensions to Cadetta paths");
            }
        }

        /// <summary>Removes the registration written by <see cref="Register"/>.</summary>
        [ComUnregisterFunction]
        public static void Unregister(Type t)
        {
            Registry.LocalMachine.DeleteSubKey(@"SOFTWARE\SolidWorks\Addins\{" + AddInGuid + "}", false);
        }
    }
}

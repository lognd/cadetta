// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
using System.Collections.Generic;
using Cadetta.SolidWorks.AddIn.Client;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.AddIn.Pane
{
    /// <summary>Everything the task pane shows, computed without any UI or COM so it is testable.</summary>
    public sealed class PaneModel
    {
        private readonly CadettaCli _cli;
        private readonly Log _log;
        private IReadOnlyList<NameRow> _rows = new List<NameRow>();

        /// <summary>Creates a model over a CLI client.</summary>
        public PaneModel(CadettaCli cli, Log log)
        {
            _cli = cli;
            _log = log;
        }

        /// <summary>The adapter the engineer chose in the pane, or null for the first.</summary>
        public string? ChosenAdapterId { get; set; }

        /// <summary>The project found for the loaded document, or null.</summary>
        public ProjectInfo? Project { get; private set; }

        /// <summary>One-line status or error for the pane's header.</summary>
        public string Status { get; private set; } = "No document";

        /// <summary>The document component's rows before any selection filter.</summary>
        public IReadOnlyList<NameRow> Rows => _rows;

        /// <summary>Loads the pane for <paramref name="documentPath"/> (null for no document): discover, run `names`, keep the component's rows.</summary>
        public bool Load(string? documentPath)
        {
            _rows = new List<NameRow>();
            Project = null;
            if (string.IsNullOrEmpty(documentPath))
            {
                Status = "No saved document";
                _log.Info("pane: no saved document");
                return false;
            }
            Project = ProjectDiscovery.Find(documentPath!, _log);
            if (Project == null)
            {
                Status = "No cadetta.toml above " + documentPath;
                return false;
            }
            var adapter = Project.Pick(ChosenAdapterId);
            if (adapter == null)
            {
                Status = "cadetta.toml has no [[adapter]] with name = \"solidworks\"";
                _log.Warn("pane: " + Status);
                return false;
            }
            var result = _cli.Names(Project, adapter);
            if (!result.IsOk)
            {
                Status = result.Error!;
                return false;
            }
            var component = PathFilter.ComponentOf(documentPath!);
            _rows = PathFilter.ForComponent(result.Value!.Names, component);
            Status = _rows.Count + " path(s) for " + component + " (adapter " + adapter.Id + ")";
            _log.Info("pane: " + Status);
            return true;
        }

        /// <summary>The rows to display for the current dimension selection.</summary>
        public IReadOnlyList<NameRow> View(SelectedDimension? selection)
        {
            var view = PathFilter.ForSelection(_rows, selection);
            _log.Debug("pane: view " + view.Count + " of " + _rows.Count + (selection == null ? " (no selection)" : " for " + selection.Kind));
            return view;
        }
    }
}

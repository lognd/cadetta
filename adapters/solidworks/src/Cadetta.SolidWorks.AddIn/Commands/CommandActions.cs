// frob:ticket 01M54BDDBT01RBBDS3AQ8ZNA4K
// frob:ticket 01M54EDNCXH7QFEEGJQNAE1M1N
using System.Collections.Generic;
using Cadetta.SolidWorks.AddIn.Client;
using Cadetta.SolidWorks.Busy;
using Cadetta.SolidWorks.Commands;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Push;

namespace Cadetta.SolidWorks.AddIn.Commands
{
    /// <summary>The add-in actions behind the menu and task pane buttons, free of COM and UI: gate, run, report as a <see cref="CommandOutcome"/>.</summary>
    public sealed class CommandActions
    {
        private readonly CadettaCommands _cli;
        private readonly CadettaCli? _frames;
        private readonly Log _log;

        /// <summary>Creates the actions over a CLI command client and the read client that supplies `cadetta frames` (null disables Assign array).</summary>
        public CommandActions(CadettaCommands cli, Log log, CadettaCli? frames = null)
        {
            _cli = cli;
            _log = log;
            _frames = frames;
        }

        /// <summary>True while a `cadetta sync` started by this add-in is running (R009).</summary>
        public bool SyncRunning { get; private set; }

        /// <summary>Links the selected dimension to the global named for <paramref name="path"/> (the name comes from `cadetta names`); gated by R009.</summary>
        public CommandOutcome Link(IPushDocument doc, IBusyProbe probe, string? dimension, string? path)
        {
            if (string.IsNullOrEmpty(dimension) || string.IsNullOrEmpty(path))
            {
                return Missing("select a dimension and a path row to link");
            }
            var gate = DocumentGate.Check(doc, probe, SyncRunning, _log);
            if (gate != null)
            {
                return gate;
            }
            return LinkCommands.Link(doc, dimension!, NameMangler.Mangle(path!), _log);
        }

        /// <summary>Removes the link on the selected dimension; gated by R009.</summary>
        public CommandOutcome Unlink(IPushDocument doc, IBusyProbe probe, string? dimension)
        {
            if (string.IsNullOrEmpty(dimension))
            {
                return Missing("select a linked dimension to unlink");
            }
            var gate = DocumentGate.Check(doc, probe, SyncRunning, _log);
            if (gate != null)
            {
                return gate;
            }
            return LinkCommands.Unlink(doc, dimension!, _log);
        }

        /// <summary>Runs `cadetta sync` (or its dry run) and returns its output as the message.</summary>
        public CommandOutcome Sync(ProjectInfo? project, string? adapterId, bool dryRun, IEnumerable<string> only)
        {
            if (project == null || string.IsNullOrEmpty(adapterId))
            {
                return Missing("no cadetta project for this document");
            }
            if (SyncRunning)
            {
                return CommandOutcome.Refused("a sync is already running", DocumentGate.Code);
            }
            SyncRunning = true;
            _log.Info("sync " + (dryRun ? "dry run" : "run") + " started");
            try
            {
                return Report(_cli.Sync(project, adapterId!, dryRun, only));
            }
            finally
            {
                SyncRunning = false;
                _log.Info("sync finished");
            }
        }

        /// <summary>Runs `cadetta why PATH` and returns the derivation as the message.</summary>
        public CommandOutcome Why(ProjectInfo? project, string? path)
        {
            if (project == null || string.IsNullOrEmpty(path))
            {
                return Missing("select a path row to explain");
            }
            return Report(_cli.Why(project, path!));
        }

        /// <summary>Assign array: gives the selected instances the references of the unique nearest instance frames of the array named by the selected pane path (`cadetta frames`); gated by R009.</summary>
        public CommandOutcome AssignArray(ProjectInfo? project, IPushDocument doc, IBusyProbe probe, IReadOnlyList<SelectedInstance> selected, IComponentReferences refs, string? path)
        {
            if (project == null || _frames == null)
            {
                return Missing("no cadetta project for this document");
            }
            var prefix = string.IsNullOrEmpty(path) ? null : ArrayAssign.ArrayPrefix(path!);
            if (prefix == null)
            {
                return Missing("select the array's placement row (for example pkg::Mount/feet[1].place) in the pane");
            }
            var gate = DocumentGate.Check(doc, probe, SyncRunning, _log);
            if (gate != null)
            {
                return gate;
            }
            var frames = _frames.Frames(project);
            if (!frames.IsOk)
            {
                return CommandOutcome.Refused(frames.Error!);
            }
            var report = frames.Value!;
            var candidates = report.ArrayCandidates(prefix);
            _log.Info("assign array " + prefix + ": " + candidates.Count + " candidate frame(s), " + selected.Count + " selected");
            return ArrayAssign.Assign(selected, candidates, ArrayAssign.DefaultMaxDistance, ArrayAssign.TieEpsilon(report.Precision), refs, _log);
        }

        private CommandOutcome Missing(string message)
        {
            _log.Warn("command refused: " + message);
            return CommandOutcome.Refused(message);
        }

        private static CommandOutcome Report(CliResult<CommandText> result)
            => result.IsOk ? CommandOutcome.Ok(result.Value!.Text.Trim()) : CommandOutcome.Refused(result.Error!);
    }
}

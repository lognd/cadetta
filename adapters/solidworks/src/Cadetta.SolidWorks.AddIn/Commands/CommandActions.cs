// frob:ticket 01M54BDDBT01RBBDS3AQ8ZNA4K
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
        private readonly Log _log;

        /// <summary>Creates the actions over a CLI command client.</summary>
        public CommandActions(CadettaCommands cli, Log log)
        {
            _cli = cli;
            _log = log;
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

        private CommandOutcome Missing(string message)
        {
            _log.Warn("command refused: " + message);
            return CommandOutcome.Refused(message);
        }

        private static CommandOutcome Report(CliResult<CommandText> result)
            => result.IsOk ? CommandOutcome.Ok(result.Value!.Text.Trim()) : CommandOutcome.Refused(result.Error!);
    }
}

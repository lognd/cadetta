// frob:ticket 01M4VB7VXXHJ6TKHF6J2STC1FE
using System.Collections.Generic;
using Cadetta.SolidWorks.AddIn.Client;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.AddIn.Commands
{
    /// <summary>The write verbs of the `cadetta` CLI the add-in drives: bind, sync and why; never edits bindings or the .cdt itself (solidworks.md 3).</summary>
    public sealed class CadettaCommands
    {
        private readonly ICommandRunner _runner;
        private readonly string _exe;
        private readonly Log _log;

        /// <summary>Creates a command client running <paramref name="exe"/> through <paramref name="runner"/>.</summary>
        public CadettaCommands(ICommandRunner runner, string? exe, Log log)
        {
            _runner = runner;
            _exe = string.IsNullOrEmpty(exe) ? CadettaCli.DefaultExecutable : exe!;
            _log = log;
        }

        /// <summary>Builds the `bind` argument list for one explicit locator (config-cli.md 3).</summary>
        public static List<string> BindArgs(string path, string adapterId, string locator, string kind, string? mode, string? component, bool remove)
        {
            var args = new List<string> { "bind", path, "--adapter", adapterId, "--locator", locator };
            if (remove)
            {
                args.Insert(1, "--remove");
                return args;
            }
            args.Add("--kind");
            args.Add(kind);
            if (!string.IsNullOrEmpty(mode))
            {
                args.Add("--mode");
                args.Add(mode!);
            }
            if (!string.IsNullOrEmpty(component))
            {
                args.Add("--component");
                args.Add(component!);
            }
            args.Add("--format");
            args.Add("json");
            return args;
        }

        /// <summary>Builds the `sync` argument list; <paramref name="only"/> restricts it to path globs.</summary>
        public static List<string> SyncArgs(string adapterId, bool dryRun, IEnumerable<string> only)
        {
            var args = new List<string> { "sync", adapterId };
            if (dryRun)
            {
                args.Add("--dry-run");
            }
            foreach (var glob in only)
            {
                args.Add("--only");
                args.Add(glob);
            }
            args.Add("--format");
            args.Add("json");
            return args;
        }

        /// <summary>Adds one explicit locator for <paramref name="path"/> to the bindings file.</summary>
        public CliResult<CommandText> Bind(ProjectInfo project, string adapterId, string path, string locator, string kind, string? mode, string? component)
            => Run(project, BindArgs(path, adapterId, locator, kind, mode, component, false));

        /// <summary>Removes one explicit locator for <paramref name="path"/> from the bindings file.</summary>
        public CliResult<CommandText> Unbind(ProjectInfo project, string adapterId, string path, string locator)
            => Run(project, BindArgs(path, adapterId, locator, "", null, null, true));

        /// <summary>Runs `sync`, or `sync --dry-run` to preview what it would push.</summary>
        public CliResult<CommandText> Sync(ProjectInfo project, string adapterId, bool dryRun, IEnumerable<string> only)
            => Run(project, SyncArgs(adapterId, dryRun, only));

        /// <summary>Runs `why PATH` for the upstream derivation of one path.</summary>
        public CliResult<CommandText> Why(ProjectInfo project, string path)
            => Run(project, new List<string> { "why", path, "--format", "json" });

        private CliResult<CommandText> Run(ProjectInfo project, List<string> args)
        {
            _log.Info("cadetta " + string.Join(" ", args) + " in " + project.Root);
            var output = _runner.Run(_exe, args, project.Root, out var startError);
            if (output == null)
            {
                _log.Error("cadetta " + args[0] + " did not start: " + startError);
                return CliResult<CommandText>.Fail(startError);
            }
            if (output.ExitCode != 0)
            {
                var detail = (output.Stderr + output.Stdout).Trim();
                _log.Warn("cadetta " + args[0] + " exit " + output.ExitCode + ": " + detail);
                return CliResult<CommandText>.Fail("cadetta " + args[0] + " failed (exit " + output.ExitCode + "): " + detail);
            }
            return CliResult<CommandText>.Ok(new CommandText(output.Stdout));
        }
    }

    /// <summary>The captured standard output of a successful command, shown to the user as is.</summary>
    public sealed class CommandText
    {
        /// <summary>Creates a result holding <paramref name="text"/>.</summary>
        public CommandText(string text)
        {
            Text = text;
        }

        /// <summary>Standard output.</summary>
        public string Text { get; }
    }
}

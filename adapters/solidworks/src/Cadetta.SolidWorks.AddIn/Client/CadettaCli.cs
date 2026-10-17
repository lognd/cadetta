// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
using System.Collections.Generic;
using System.Linq;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.AddIn.Client
{
    /// <summary>Client of the `cadetta` command line (solidworks.md 3); never computes names or edits files itself.</summary>
    public sealed class CadettaCli
    {
        /// <summary>Default executable: `cadetta` on PATH.</summary>
        public const string DefaultExecutable = "cadetta";

        private readonly ICommandRunner _runner;
        private readonly string _exe;
        private readonly Log _log;

        /// <summary>Creates a client running <paramref name="exe"/> through <paramref name="runner"/>.</summary>
        public CadettaCli(ICommandRunner runner, string? exe, Log log)
        {
            _runner = runner;
            _exe = string.IsNullOrEmpty(exe) ? DefaultExecutable : exe!;
            _log = log;
        }

        /// <summary>Runs `cadetta names ADAPTER --format json` in the project and returns that adapter's rows.</summary>
        public CliResult<NamesAdapter> Names(ProjectInfo project, AdapterRef adapter)
        {
            var args = new List<string> { "names", adapter.Id, "--format", "json" };
            _log.Info("cadetta names " + adapter.Id + " in " + project.Root);
            var output = _runner.Run(_exe, args, project.Root, out var startError);
            if (output == null)
            {
                return CliResult<NamesAdapter>.Fail(startError);
            }
            if (output.ExitCode != 0)
            {
                var detail = output.Stderr.Trim();
                _log.Warn("cadetta names exit " + output.ExitCode + ": " + detail);
                return CliResult<NamesAdapter>.Fail("cadetta names failed (exit " + output.ExitCode + "): " + detail);
            }
            var report = NamesReport.Parse(output.Stdout);
            if (report == null)
            {
                _log.Error("cadetta names: output is not valid JSON");
                return CliResult<NamesAdapter>.Fail("cadetta names printed output that is not JSON");
            }
            var mine = report.Adapters.FirstOrDefault(a => a.Id == adapter.Id);
            if (mine == null)
            {
                _log.Warn("cadetta names: adapter " + adapter.Id + " missing from the report");
                return CliResult<NamesAdapter>.Fail("cadetta names did not report adapter " + adapter.Id);
            }
            _log.Info("cadetta names: " + mine.Names.Count + " path(s), " + report.Diagnostics.Count + " diagnostic(s)");
            return CliResult<NamesAdapter>.Ok(mine);
        }
    }
}

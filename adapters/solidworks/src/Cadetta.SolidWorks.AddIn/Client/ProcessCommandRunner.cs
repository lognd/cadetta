// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Text;
using System.Threading.Tasks;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.AddIn.Client
{
    /// <summary>ICommandRunner over System.Diagnostics.Process (no shell, no console window).</summary>
    public sealed class ProcessCommandRunner : ICommandRunner
    {
        private readonly Log _log;
        private readonly int _timeoutMs;

        /// <summary>Creates a runner that kills a process still running after <paramref name="timeoutMs"/>.</summary>
        public ProcessCommandRunner(Log log, int timeoutMs = 60000)
        {
            _log = log;
            _timeoutMs = timeoutMs;
        }

        /// <inheritdoc />
        public ProcessOutput? Run(string exe, IReadOnlyList<string> args, string workingDir, out string startError)
        {
            startError = "";
            var psi = new ProcessStartInfo(exe)
            {
                WorkingDirectory = workingDir,
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                StandardOutputEncoding = new UTF8Encoding(false),
                StandardErrorEncoding = new UTF8Encoding(false),
            };
            psi.Arguments = JoinArgs(args);
            _log.Debug("exec " + exe + " " + psi.Arguments + " in " + workingDir);
            try
            {
                using (var p = Process.Start(psi)!)
                {
                    var err = p.StandardError.ReadToEndAsync();
                    var outp = p.StandardOutput.ReadToEndAsync();
                    if (!p.WaitForExit(_timeoutMs))
                    {
                        p.Kill();
                        startError = exe + " timed out after " + _timeoutMs / 1000 + "s";
                        _log.Error(startError);
                        return null;
                    }
                    Task.WaitAll(err, outp);
                    _log.Debug("exec exit " + p.ExitCode);
                    return new ProcessOutput(p.ExitCode, outp.Result, err.Result);
                }
            }
            catch (Win32Exception e)
            {
                startError = "cannot start " + exe + ": " + e.Message;
                _log.Error(startError);
                return null;
            }
            catch (InvalidOperationException e)
            {
                startError = "cannot start " + exe + ": " + e.Message;
                _log.Error(startError);
                return null;
            }
        }

        /// <summary>Joins arguments with Windows command-line quoting.</summary>
        public static string JoinArgs(IReadOnlyList<string> args)
        {
            var sb = new StringBuilder();
            foreach (var a in args)
            {
                if (sb.Length > 0)
                {
                    sb.Append(' ');
                }
                if (a.Length > 0 && a.IndexOfAny(new[] { ' ', '\t', '"' }) < 0)
                {
                    sb.Append(a);
                }
                else
                {
                    sb.Append('"').Append(a.Replace("\\", "\\\\").Replace("\"", "\\\"")).Append('"');
                }
            }
            return sb.ToString();
        }
    }
}

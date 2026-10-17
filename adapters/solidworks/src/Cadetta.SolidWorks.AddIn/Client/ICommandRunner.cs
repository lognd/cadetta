// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
using System.Collections.Generic;

namespace Cadetta.SolidWorks.AddIn.Client
{
    /// <summary>What a finished process produced.</summary>
    public sealed class ProcessOutput
    {
        /// <summary>Creates a process output.</summary>
        public ProcessOutput(int exitCode, string stdout, string stderr)
        {
            ExitCode = exitCode;
            Stdout = stdout;
            Stderr = stderr;
        }

        /// <summary>Process exit code.</summary>
        public int ExitCode { get; }

        /// <summary>Captured standard output.</summary>
        public string Stdout { get; }

        /// <summary>Captured standard error.</summary>
        public string Stderr { get; }
    }

    /// <summary>Runs a process to completion; the seam that keeps CadettaCli testable without a cadetta binary.</summary>
    public interface ICommandRunner
    {
        /// <summary>Runs <paramref name="exe"/> with <paramref name="args"/> in <paramref name="workingDir"/>; null when it could not be started (message in <paramref name="startError"/>).</summary>
        ProcessOutput? Run(string exe, IReadOnlyList<string> args, string workingDir, out string startError);
    }
}

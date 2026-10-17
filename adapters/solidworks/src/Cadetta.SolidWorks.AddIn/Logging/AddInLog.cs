// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
using System;
using System.IO;
using System.Text;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.AddIn.Logging
{
    /// <summary>Builds the add-in logger: %LOCALAPPDATA%\cadetta\addin.log filtered by CADETTA_LOG (solidworks.md 1).</summary>
    public static class AddInLog
    {
        /// <summary>Name of the add-in log file.</summary>
        public const string FileName = "addin.log";

        /// <summary>The log file path under <paramref name="localAppData"/>.</summary>
        public static string PathUnder(string localAppData) =>
            Path.Combine(localAppData, "cadetta", FileName);

        /// <summary>Opens the log for append (shared, so SOLIDWORKS and a tailing viewer coexist); falls back to a null logger when the file cannot be opened.</summary>
        public static Log Open(string? localAppData, string? filter)
        {
            if (string.IsNullOrEmpty(localAppData))
            {
                return Log.Null();
            }
            try
            {
                var path = PathUnder(localAppData!);
                Directory.CreateDirectory(Path.GetDirectoryName(path)!);
                var stream = new FileStream(path, FileMode.Append, FileAccess.Write, FileShare.ReadWrite);
                var writer = new StreamWriter(stream, new UTF8Encoding(false));
                return new Log(writer, Log.ParseFilter(filter));
            }
            catch (IOException)
            {
                return Log.Null();
            }
            catch (UnauthorizedAccessException)
            {
                return Log.Null();
            }
        }

        /// <summary>Opens the log from the process environment.</summary>
        public static Log OpenDefault() =>
            Open(Environment.GetEnvironmentVariable("LOCALAPPDATA"), Environment.GetEnvironmentVariable("CADETTA_LOG"));
    }
}

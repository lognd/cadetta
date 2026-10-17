// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
// frob:ticket 01M54A4VN81506B2GPW4FG4VVR
using System;
using Microsoft.Win32;

namespace Cadetta.SolidWorks.AddIn
{
    /// <summary>The add-in settings (solidworks.md 3): the cadetta executable, default `cadetta` on PATH.</summary>
    public static class AddInSettings
    {
        private const string Key = @"Software\Cadetta\SolidWorks";

        /// <summary>The executable: CADETTA_EXE, else HKCU\Software\Cadetta\SolidWorks\Executable, else null (the client then uses `cadetta`).</summary>
        public static string? Executable()
        {
            var env = Environment.GetEnvironmentVariable("CADETTA_EXE");
            if (!string.IsNullOrEmpty(env))
            {
                return env;
            }
            using (var key = Registry.CurrentUser.OpenSubKey(Key))
            {
                return key?.GetValue("Executable") as string;
            }
        }

        /// <summary>The log filter: CADETTA_LOG, else HKCU\Software\Cadetta\SolidWorks\Log (a running SOLIDWORKS cannot be given a new environment), else null (warn).</summary>
        public static string? LogFilter()
        {
            var env = Environment.GetEnvironmentVariable("CADETTA_LOG");
            if (!string.IsNullOrEmpty(env))
            {
                return env;
            }
            using (var key = Registry.CurrentUser.OpenSubKey(Key))
            {
                return key?.GetValue("Log") as string;
            }
        }
    }
}

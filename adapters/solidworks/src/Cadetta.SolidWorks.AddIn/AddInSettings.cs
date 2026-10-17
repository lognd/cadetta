// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
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
    }
}

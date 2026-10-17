using System;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.Save
{
    /// <summary>
    /// The `save` step of a drive sync: writes only modified documents whose
    /// rebuild did not fail and records the digest of the saved bytes
    /// (adapters.md 2.2, 6.3).
    /// </summary>
    public sealed class SaveService
    {
        private readonly IFileReader _reader;
        private readonly Log _log;

        /// <summary>Creates a service reading saved bytes through <paramref name="reader"/>.</summary>
        public SaveService(IFileReader reader, Log log)
        {
            _reader = reader;
            _log = log;
        }

        /// <summary>Saves modified, cleanly rebuilt documents unless the mode is never or a locator failed.</summary>
        public SaveOutcome Run(SaveMode mode, bool anyLocatorFailed, ISaveTarget[] targets)
        {
            var outcome = new SaveOutcome();
            if (mode == SaveMode.Never)
            {
                _log.Info("save skipped: mode is never, documents stay unsaved");
                outcome.Skipped = true;
                return outcome;
            }
            if (anyLocatorFailed)
            {
                _log.Warn("save skipped: a locator failed");
                outcome.Skipped = true;
                outcome.Diagnostics.Add("save skipped: a locator failed");
                return outcome;
            }
            foreach (var t in targets)
            {
                if (!t.Modified)
                {
                    _log.Debug("save: " + t.Path + " unmodified, not written");
                    continue;
                }
                if (t.RebuildFailed)
                {
                    _log.Warn("save: " + t.Path + " rebuild failed, not written");
                    outcome.Diagnostics.Add(t.Path + ": not saved, rebuild failed");
                    continue;
                }
                var error = t.Save();
                if (error != null)
                {
                    _log.Error("save: " + t.Path + " failed: " + error);
                    outcome.Diagnostics.Add(t.Path + ": save failed: " + error);
                    continue;
                }
                var digest = Blake3.Digest(_reader.ReadAllBytes(t.Path));
                _log.Info("save: " + t.Path + " saved " + digest);
                outcome.Files.Add(new Protocol.FileDigest { Path = t.Path, Digest = digest });
            }
            return outcome;
        }
    }
}

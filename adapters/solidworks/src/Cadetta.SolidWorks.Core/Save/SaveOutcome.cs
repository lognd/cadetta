using System.Collections.Generic;

namespace Cadetta.SolidWorks.Save
{
    /// <summary>Result of a save step: files written with digests, and why anything was not.</summary>
    public sealed class SaveOutcome
    {
        /// <summary>Files saved, with `blake3:` digests of the saved bytes.</summary>
        public List<Protocol.FileDigest> Files { get; } = new List<Protocol.FileDigest>();

        /// <summary>Human-readable diagnostics (skips and save failures).</summary>
        public List<string> Diagnostics { get; } = new List<string>();

        /// <summary>True when the step was skipped as a whole (mode never, or a locator failed).</summary>
        public bool Skipped { get; set; }
    }
}

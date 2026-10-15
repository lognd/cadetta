using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Save;
using SolidWorks.Interop.sldworks;
using SolidWorks.Interop.swconst;

namespace Cadetta.SolidWorks.Com.Save
{
    /// <summary>ISaveTarget over a COM ModelDoc2; the session layer sets the modified and rebuild flags.</summary>
    public sealed class SwComSaveTarget : ISaveTarget
    {
        private readonly IModelDoc2 _doc;
        private readonly Log _log;

        /// <summary>Wraps <paramref name="doc"/> at <paramref name="path"/>.</summary>
        public SwComSaveTarget(IModelDoc2 doc, string path, Log log)
        {
            _doc = doc;
            _log = log;
            Path = path;
        }

        /// <summary>Path of the document file.</summary>
        public string Path { get; }

        /// <summary>True when the adapter pushed a change into this document.</summary>
        public bool Modified { get; set; }

        /// <summary>True when the last rebuild of this document failed.</summary>
        public bool RebuildFailed { get; set; }

        /// <summary>Saves silently via Save3; returns an error message, or null on success.</summary>
        public string? Save()
        {
            int errors = 0;
            int warnings = 0;
            var ok = _doc.Save3((int)swSaveAsOptions_e.swSaveAsOptions_Silent, ref errors, ref warnings);
            _log.Debug("Save3 " + Path + ": ok=" + ok + " errors=" + errors + " warnings=" + warnings);
            return ok ? null : "Save3 failed with error flags " + errors;
        }
    }
}

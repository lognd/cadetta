using System;
using System.IO;
using Cadetta.SolidWorks.Com.Push;
using Cadetta.SolidWorks.Com.Save;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Push;
using Cadetta.SolidWorks.Save;
using Cadetta.SolidWorks.Session;
using SolidWorks.Interop.sldworks;
using SolidWorks.Interop.swconst;

namespace Cadetta.SolidWorks.Com.Session
{
    // frob:ticket 01M5496CQE8FPX6F7PD9Y6EMX0
    /// <summary>IPushHost over a SOLIDWORKS application: reuses an open copy, otherwise opens silently for writing.</summary>
    public sealed class SwComPushHost : IPushHost
    {
        private readonly ISldWorks _sw;
        private readonly Log _log;

        /// <summary>Creates a host over <paramref name="sw"/>.</summary>
        public SwComPushHost(ISldWorks sw, Log log)
        {
            _sw = sw;
            _log = log;
        }

        /// <summary>Opens <paramref name="file"/> for a push, reusing an already open copy.</summary>
        public IPushHandle Open(string file)
        {
            var model = _sw.GetOpenDocumentByName(file) as IModelDoc2;
            var opened = false;
            if (model == null)
            {
                int errors = 0, warnings = 0;
                model = _sw.OpenDoc6(file, TypeOf(file), (int)swOpenDocOptions_e.swOpenDocOptions_Silent, "", ref errors, ref warnings);
                if (model == null)
                {
                    _log.Error("OpenDoc6 failed for " + file + ": errors=" + errors);
                    throw new InvalidOperationException("OpenDoc6 failed for " + file + ": errors=" + errors);
                }
                opened = true;
                _log.Info("opened " + file + " for push (warnings=" + warnings + ")");
            }
            return new Handle(_sw, model, file, opened, _log);
        }

        private static int TypeOf(string path)
        {
            switch (Path.GetExtension(path).ToLowerInvariant())
            {
                case ".sldasm": return (int)swDocumentTypes_e.swDocASSEMBLY;
                case ".slddrw": return (int)swDocumentTypes_e.swDocDRAWING;
                default: return (int)swDocumentTypes_e.swDocPART;
            }
        }

        private sealed class Handle : IPushHandle
        {
            private readonly ISldWorks _sw;
            private readonly IModelDoc2 _model;
            private readonly string _file;
            private readonly bool _opened;
            private readonly Log _log;
            private readonly SwComPushDocument _doc;
            private readonly SwComSaveTarget _target;

            public Handle(ISldWorks sw, IModelDoc2 model, string file, bool opened, Log log)
            {
                _sw = sw;
                _model = model;
                _file = file;
                _opened = opened;
                _log = log;
                _doc = new SwComPushDocument(model, file, log);
                _target = new SwComSaveTarget(model, file, log);
            }

            public IPushDocument Document => _doc;

            public ISaveTarget Target => _target;

            public void Settle(bool modified, bool rebuildFailed)
            {
                _target.Modified = modified;
                _target.RebuildFailed = rebuildFailed;
            }

            public void Dispose()
            {
                if (_opened)
                {
                    _log.Debug("closing " + _file);
                    _sw.CloseDoc(_model.GetTitle());
                }
            }
        }
    }
}

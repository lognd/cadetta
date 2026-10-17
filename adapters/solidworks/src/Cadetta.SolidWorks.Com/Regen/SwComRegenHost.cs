using System;
using System.Collections.Generic;
using System.IO;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Regen;
using SolidWorks.Interop.sldworks;
using SolidWorks.Interop.swconst;

namespace Cadetta.SolidWorks.Com.Regen
{
    // frob:ticket 01M4SV2TVM50WJRZ42ZHXVD96N
    /// <summary>IRegenHost over a SOLIDWORKS application: reuses an open copy, otherwise opens silently.</summary>
    public sealed class SwComRegenHost : IRegenHost
    {
        private readonly ISldWorks _sw;
        private readonly Log _log;

        /// <summary>Creates a host over <paramref name="sw"/>.</summary>
        public SwComRegenHost(ISldWorks sw, Log log)
        {
            _sw = sw;
            _log = log;
        }

        /// <summary>Opens <paramref name="file"/> for a regen, reusing an already open copy.</summary>
        public IRegenHandle Open(string file)
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
                _log.Info("opened " + file + " for regen (warnings=" + warnings + ")");
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

        private sealed class Handle : IRegenHandle
        {
            private readonly ISldWorks _sw;
            private readonly IModelDoc2 _model;
            private readonly string _file;
            private readonly bool _opened;
            private readonly Log _log;

            public Handle(ISldWorks sw, IModelDoc2 model, string file, bool opened, Log log)
            {
                _sw = sw;
                _model = model;
                _file = file;
                _opened = opened;
                _log = log;
                Document = new Doc(model, file, log);
            }

            public IRegenDocument Document { get; }

            public void Dispose()
            {
                if (_opened)
                {
                    _log.Debug("closing " + _file);
                    _sw.CloseDoc(_model.GetTitle());
                }
            }
        }

        private sealed class Doc : IRegenDocument
        {
            private readonly IModelDoc2 _model;
            private readonly Log _log;

            public Doc(IModelDoc2 model, string path, Log log)
            {
                _model = model;
                Path = path;
                _log = log;
            }

            public string Path { get; }

            public bool ReadOnly => _model.IsOpenedReadOnly();

            public bool UserDirty => _model.GetSaveFlag();

            public IReadOnlyList<RebuildError> Rebuild()
            {
                var ok = _model.ForceRebuild3(false);
                var errors = new List<RebuildError>();
                for (var f = _model.FirstFeature() as IFeature; f != null; f = f.GetNextFeature() as IFeature)
                {
                    bool warn;
                    var code = f.GetErrorCode2(out warn);
                    if (code != 0 && !warn)
                    {
                        errors.Add(new RebuildError { Feature = f.Name, Code = code, Message = "feature error code " + code });
                    }
                }
                if (!ok && errors.Count == 0)
                {
                    errors.Add(new RebuildError { Message = "ForceRebuild3 reported failure" });
                }
                _log.Debug("rebuild ok=" + ok + " feature errors=" + errors.Count);
                return errors;
            }
        }
    }
}

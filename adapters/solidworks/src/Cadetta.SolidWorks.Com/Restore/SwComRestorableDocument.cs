using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Restore;
using SolidWorks.Interop.sldworks;

namespace Cadetta.SolidWorks.Com.Restore
{
    /// <summary>IRestorableDocument over a SOLIDWORKS model: locators are dimension names such as D1@Sketch1.</summary>
    public sealed class SwComRestorableDocument : IRestorableDocument
    {
        private readonly IModelDoc2 _model;
        private readonly Log _log;

        /// <summary>Creates an adapter over <paramref name="model"/>.</summary>
        public SwComRestorableDocument(IModelDoc2 model, Log log)
        {
            _model = model;
            _log = log;
        }

        /// <summary>Reads the system value (metres or radians) of the named dimension.</summary>
        public double? GetValue(string locator)
        {
            var dim = (IDimension)_model.Parameter(locator);
            if (dim == null)
            {
                _log.Debug("dimension not found: " + locator);
                return null;
            }
            return dim.SystemValue;
        }

        /// <summary>Sets the system value of the named dimension; false when it is missing or SOLIDWORKS refuses.</summary>
        public bool SetValue(string locator, double value)
        {
            var dim = (IDimension)_model.Parameter(locator);
            if (dim == null)
            {
                _log.Warn("dimension not found: " + locator);
                return false;
            }
            var status = dim.SetSystemValue3(value, 2, null);
            _log.Debug("set " + locator + " status " + status);
            return status == 0;
        }

        /// <summary>Forces a full rebuild and reports whether it finished without errors.</summary>
        public bool Rebuild()
        {
            var ok = _model.ForceRebuild3(false);
            _log.Debug("rebuild ok: " + ok);
            return ok;
        }
    }
}

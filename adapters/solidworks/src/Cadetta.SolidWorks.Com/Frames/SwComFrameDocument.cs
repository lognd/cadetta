using System;
using Cadetta.SolidWorks.Frames;
using Cadetta.SolidWorks.Logging;
using SolidWorks.Interop.sldworks;

namespace Cadetta.SolidWorks.Com.Frames
{
    // frob:ticket 01M4SWMDXKYAFXR9GEKVTE7XA4
    /// <summary>IFrameDocument over a SOLIDWORKS model: locators are coordinate system or reference plane feature names.</summary>
    public sealed class SwComFrameDocument : IFrameDocument
    {
        private readonly IModelDoc2 _model;
        private readonly Log _log;

        /// <summary>Creates an adapter over <paramref name="model"/>.</summary>
        public SwComFrameDocument(IModelDoc2 model, Log log)
        {
            _model = model;
            _log = log;
        }

        /// <summary>Resolves a coordinate system via its transform, or a reference plane via its feature; null when neither exists.</summary>
        public FrameReading? ReadFrame(string locator)
        {
            var feature = (IFeature)_model.FeatureByName(locator);
            if (feature == null)
            {
                _log.Debug("frame feature not found: " + locator);
                return null;
            }
            var type = feature.GetTypeName2();
            if (type == "CoordSys")
            {
                var transform = (IMathTransform)_model.Extension.GetCoordinateSystemTransformByName(locator);
                if (transform == null)
                {
                    _log.Warn("coordinate system has no transform: " + locator);
                    return null;
                }
                return new FrameReading(FrameKind.CoordinateSystem, FrameTransform.FromSwArray((double[])transform.ArrayData));
            }
            if (type == "RefPlane")
            {
                var plane = (IRefPlane)feature.GetSpecificFeature2();
                var transform = (IMathTransform)plane.Transform;
                return new FrameReading(FrameKind.Plane, FrameTransform.FromSwArray((double[])transform.ArrayData));
            }
            _log.Warn("feature " + locator + " is " + type + ", not a frame");
            return null;
        }

        /// <summary>
        /// Creates the coordinate system, or moves an existing one by deleting and
        /// recreating it under the same name (the API has no in-place update).
        /// </summary>
        public bool CreateOrUpdateCoordinateSystem(string name, FrameTransform metres)
        {
            var existing = (IFeature)_model.FeatureByName(name);
            if (existing != null)
            {
                if (!_model.Extension.SelectByID2(name, "COORDSYS", 0, 0, 0, false, 0, null, 0) || !_model.Extension.DeleteSelection2(0))
                {
                    _log.Warn("could not remove coordinate system " + name + " for update");
                    return false;
                }
            }
            var angles = metres.ToEulerXyz();
            var created = (IFeature)_model.FeatureManager.CreateCoordinateSystemUsingNumericalValues(
                true, metres.X, metres.Y, metres.Z, true, angles[0], angles[1], angles[2]);
            if (created == null)
            {
                _log.Warn("coordinate system creation refused: " + name);
                return false;
            }
            created.Name = name;
            _log.Info("coordinate system " + name + " written");
            return true;
        }
    }
}

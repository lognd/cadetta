using Cadetta.SolidWorks.Frames;
using Cadetta.SolidWorks.Logging;
using SolidWorks.Interop.sldworks;

namespace Cadetta.SolidWorks.Com.Frames
{
    // frob:ticket 01M4SWMDXKYAFXR9GEKVTE7XA4
    /// <summary>ISketchDocument over a SOLIDWORKS model: locators are dimension names such as D1@Sketch1.</summary>
    public sealed class SwComSketchDocument : ISketchDocument
    {
        private readonly IModelDoc2 _model;
        private readonly Log _log;

        /// <summary>Creates an adapter over <paramref name="model"/>.</summary>
        public SwComSketchDocument(IModelDoc2 model, Log log)
        {
            _model = model;
            _log = log;
        }

        /// <summary>
        /// Finds the sketch named after the `@` suffix and reports its plane. The
        /// measured-from answer is `unknown` unless the dimension has no
        /// driving sketch reference (see the README limitation).
        /// </summary>
        public SketchFacts? ReadSketch(string dimensionLocator)
        {
            var at = dimensionLocator.IndexOf('@');
            if (at < 0)
            {
                _log.Debug("no sketch in locator: " + dimensionLocator);
                return null;
            }
            var feature = (IFeature)_model.FeatureByName(dimensionLocator.Substring(at + 1));
            if (feature == null || feature.GetTypeName2() != "ProfileFeature")
            {
                _log.Debug("not a sketch: " + dimensionLocator);
                return null;
            }
            var sketch = (ISketch)feature.GetSpecificFeature2();
            var transform = (IMathTransform)sketch.ModelToSketchTransform.Inverse();
            var plane = FrameTransform.FromSwArray((double[])transform.ArrayData);
            _log.Debug("sketch " + feature.Name + " plane " + plane);
            return new SketchFacts(plane, "unknown");
        }
    }
}

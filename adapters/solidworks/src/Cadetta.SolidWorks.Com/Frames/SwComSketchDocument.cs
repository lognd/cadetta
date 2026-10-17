using System;
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
        /// Finds the sketch named after the first `@` of the dimension name
        /// (D1@Sketch1 or D1@Sketch1@Part1.Part), reports its plane, and answers
        /// measured_from from the dimension's attached entities: the sketch
        /// origin when any is the origin point, else `other`.
        /// </summary>
        public SketchFacts? ReadSketch(string dimensionLocator)
        {
            // frob:ticket 01M548FW58GD5Z00JGMAD835T5
            var at = dimensionLocator.IndexOf('@');
            if (at < 0)
            {
                _log.Debug("no sketch in locator: " + dimensionLocator);
                return null;
            }
            var rest = dimensionLocator.Substring(at + 1);
            var next = rest.IndexOf('@');
            var sketchName = next < 0 ? rest : rest.Substring(0, next);
            var feature = (IFeature)((IPartDoc)_model).FeatureByName(sketchName);
            if (feature == null || feature.GetTypeName2() != "ProfileFeature")
            {
                _log.Debug("not a sketch: " + dimensionLocator);
                return null;
            }
            var sketch = (ISketch)feature.GetSpecificFeature2();
            var transform = (IMathTransform)sketch.ModelToSketchTransform.Inverse();
            var plane = FrameTransform.FromSwArray((double[])transform.ArrayData);
            var measuredFrom = MeasuredFrom(dimensionLocator);
            _log.Debug("sketch " + feature.Name + " plane " + plane + " measured_from " + measuredFrom);
            return new SketchFacts(plane, measuredFrom);
        }

        /// <summary>Classifies the dimension's attached entities: any ISketchPoint at (0,0,0) is the sketch origin.</summary>
        private string MeasuredFrom(string dimensionLocator)
        {
            _model.ClearSelection2(true);
            try
            {
                if (!_model.Extension.SelectByID2(dimensionLocator, "DIMENSION", 0, 0, 0, false, 0, null, 0))
                {
                    _log.Debug("dimension not selectable: " + dimensionLocator);
                    return "unknown";
                }
                var display = _model.SelectionManager is ISelectionMgr mgr ? mgr.GetSelectedObject6(1, -1) as IDisplayDimension : null;
                var attached = display?.GetAnnotation() is IAnnotation ann ? ann.GetAttachedEntities3() as object[] : null;
                if (attached == null)
                {
                    return "unknown";
                }
                foreach (var entity in attached)
                {
                    if (entity is ISketchPoint p && Math.Abs(p.X) < 1e-9 && Math.Abs(p.Y) < 1e-9 && Math.Abs(p.Z) < 1e-9)
                    {
                        return SketchFacts.SketchOrigin;
                    }
                }
                return "other";
            }
            finally
            {
                _model.ClearSelection2(true);
            }
        }
    }
}

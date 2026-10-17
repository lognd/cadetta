namespace Cadetta.SolidWorks.Frames
{
    // frob:ticket 01M4SWMDXKYAFXR9GEKVTE7XA4
    /// <summary>What the document shows about the sketch holding a bound dimension.</summary>
    public sealed class SketchFacts
    {
        /// <summary>The label reported when the dimension is measured from the sketch origin.</summary>
        public const string SketchOrigin = "sketch origin";

        /// <summary>Creates facts: the sketch plane (origin and normal, metres) and what the dimension is measured from.</summary>
        public SketchFacts(FrameTransform plane, string measuredFrom)
        {
            Plane = plane;
            MeasuredFrom = measuredFrom;
        }

        /// <summary>The sketch plane in the document frame, in metres.</summary>
        public FrameTransform Plane { get; }

        /// <summary>`sketch origin` when clean, else a description of the reference.</summary>
        public string MeasuredFrom { get; }
    }

    // frob:ticket 01M4SWMDXKYAFXR9GEKVTE7XA4
    /// <summary>One open document seen as sketch dimensions and the sketches that hold them.</summary>
    public interface ISketchDocument
    {
        /// <summary>Resolves a dimension locator to its sketch's facts; null when it is not a sketch dimension.</summary>
        SketchFacts? ReadSketch(string dimensionLocator);
    }
}

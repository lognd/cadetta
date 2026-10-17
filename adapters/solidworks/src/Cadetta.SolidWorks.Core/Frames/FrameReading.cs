namespace Cadetta.SolidWorks.Frames
{
    // frob:ticket 01M4SWMDXKYAFXR9GEKVTE7XA4
    /// <summary>What kind of native feature a frame locator resolved to (solidworks.md 4).</summary>
    public enum FrameKind
    {
        /// <summary>A coordinate system feature: origin and all three axes.</summary>
        CoordinateSystem,
        /// <summary>A plane feature: only its origin and z axis are meaningful.</summary>
        Plane,
    }

    // frob:ticket 01M4SWMDXKYAFXR9GEKVTE7XA4
    /// <summary>A resolved frame: its kind and its transform in metres in the document frame.</summary>
    public sealed class FrameReading
    {
        /// <summary>Creates a reading of <paramref name="kind"/> with <paramref name="transform"/> in metres.</summary>
        public FrameReading(FrameKind kind, FrameTransform transform)
        {
            Kind = kind;
            Transform = transform;
        }

        /// <summary>The feature kind.</summary>
        public FrameKind Kind { get; }

        /// <summary>The transform in metres.</summary>
        public FrameTransform Transform { get; }
    }
}

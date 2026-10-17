namespace Cadetta.SolidWorks.Frames
{
    // frob:ticket 01M4SWMDXKYAFXR9GEKVTE7XA4
    /// <summary>One open document seen as named frames, the surface frame reads and datum creation need (adapters.md 3.4).</summary>
    public interface IFrameDocument
    {
        /// <summary>Resolves a frame locator to a coordinate system or plane feature; null when it does not resolve.</summary>
        FrameReading? ReadFrame(string locator);

        /// <summary>Creates the coordinate system <paramref name="name"/> at <paramref name="metres"/>, or moves the existing one; false when SOLIDWORKS refuses.</summary>
        bool CreateOrUpdateCoordinateSystem(string name, FrameTransform metres);
    }
}

using System.Collections.Generic;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.Frames
{
    // frob:ticket 01M4SWMDXKYAFXR9GEKVTE7XA4
    /// <summary>Reads bound frame locators into the observed IR's frames[] in the native unit (solidworks.md 4).</summary>
    public static class FrameReader
    {
        /// <summary>Reads each locator from <paramref name="doc"/>; unresolved locators are logged and left out.</summary>
        /// <param name="unitMetres">Metres per native unit (0.001 for millimetres).</param>
        public static IReadOnlyList<ObservedFrame> Read(
            IFrameDocument doc, string document, IEnumerable<string> locators, double unitMetres, Log log)
        {
            var frames = new List<ObservedFrame>();
            foreach (var locator in locators)
            {
                var reading = doc.ReadFrame(locator);
                if (reading == null)
                {
                    log.Warn("frame locator did not resolve: " + locator);
                    continue;
                }
                var native = reading.Transform.Scaled(1.0 / unitMetres);
                log.Debug("frame " + locator + " kind " + reading.Kind + " " + native);
                frames.Add(new ObservedFrame
                {
                    Name = locator,
                    Document = document,
                    Transform = native.ToRows(),
                    PlaneOnly = reading.Kind == FrameKind.Plane,
                });
            }
            return frames;
        }
    }
}

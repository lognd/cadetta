using System;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.Frames
{
    // frob:ticket 01M4SWMDXKYAFXR9GEKVTE7XA4
    /// <summary>Reads the R007 facts: is a bound sketch on its datum's plane and dimensioned from its origin (geometry.md 6, lints.md R007).</summary>
    public static class SketchObserver
    {
        /// <summary>The lint code the host raises from a non-clean fact.</summary>
        public const string Code = "R007";

        /// <summary>Distance and angle tolerance, in metres and in direction cosine, for "on the plane".</summary>
        public const double Tolerance = 1e-6;

        /// <summary>
        /// Observes the sketch of <paramref name="dimensionLocator"/> against the
        /// datum frame <paramref name="datumFrame"/>; null when the locator is not a
        /// sketch dimension or the frame does not resolve (nothing to compare).
        /// </summary>
        public static ObservedSketch? Observe(
            ISketchDocument sketches, IFrameDocument frames, string file, string dimensionLocator, string datumFrame, Log log)
        {
            var facts = sketches.ReadSketch(dimensionLocator);
            if (facts == null)
            {
                log.Debug("not a sketch dimension: " + dimensionLocator);
                return null;
            }
            var frame = frames.ReadFrame(datumFrame);
            if (frame == null)
            {
                log.Warn("datum frame did not resolve: " + datumFrame);
                return null;
            }
            var onPlane = OnPlane(facts.Plane, frame.Transform);
            if (!onPlane || facts.MeasuredFrom != SketchFacts.SketchOrigin)
            {
                log.Info(Code + " fact for " + dimensionLocator + ": on_datum_plane=" + onPlane + ", measured_from=" + facts.MeasuredFrom);
            }
            return new ObservedSketch
            {
                File = file,
                Locator = dimensionLocator,
                OnDatumPlane = onPlane,
                MeasuredFrom = facts.MeasuredFrom,
            };
        }

        /// <summary>True when the sketch plane is parallel to the frame's xy plane and contains the frame's origin.</summary>
        public static bool OnPlane(FrameTransform sketchPlane, FrameTransform datum)
        {
            var n = sketchPlane.ZAxis;
            var d = datum.ZAxis;
            var cross = new[]
            {
                n[1] * d[2] - n[2] * d[1],
                n[2] * d[0] - n[0] * d[2],
                n[0] * d[1] - n[1] * d[0],
            };
            if (Math.Abs(cross[0]) > Tolerance || Math.Abs(cross[1]) > Tolerance || Math.Abs(cross[2]) > Tolerance)
            {
                return false;
            }
            var dist = (datum.X - sketchPlane.X) * n[0] + (datum.Y - sketchPlane.Y) * n[1] + (datum.Z - sketchPlane.Z) * n[2];
            return Math.Abs(dist) <= Tolerance;
        }
    }
}

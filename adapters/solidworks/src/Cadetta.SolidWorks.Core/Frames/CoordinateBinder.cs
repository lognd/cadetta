using System;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Protocol;
using Cadetta.SolidWorks.Restore;

namespace Cadetta.SolidWorks.Frames
{
    // frob:ticket 01M4SWMDXKYAFXR9GEKVTE7XA4
    /// <summary>Pushes and reads a datum-plane sketch coordinate through its bound dimension, applying `sign` (geometry.md 6).</summary>
    public static class CoordinateBinder
    {
        /// <summary>The dimension value (metres) for the datum coordinate <paramref name="native"/>: <c>sign * native * unit</c>; sign must be 1 or -1.</summary>
        public static double ToDimension(double native, int sign, double unitMetres) => CheckSign(sign) * native * unitMetres;

        /// <summary>The datum coordinate (native unit) a dimension of <paramref name="metres"/> stands for.</summary>
        public static double FromDimension(double metres, int sign, double unitMetres) => CheckSign(sign) * metres / unitMetres;

        /// <summary>
        /// Writes the coordinate to <paramref name="locator"/>. When
        /// <paramref name="sketch"/> is not clean (off the datum plane, or not
        /// measured from the sketch origin) the value is still pushed, because
        /// the binding is the engineer's, and a warning is logged; R007 is the
        /// host's report from the read facts.
        /// </summary>
        public static LocatorStatus Push(
            IRestorableDocument doc, string locator, double native, int sign, double unitMetres,
            ObservedSketch? sketch, Log log)
        {
            var status = new LocatorStatus { Locator = locator };
            if (sketch != null && (!sketch.OnDatumPlane || sketch.MeasuredFrom != SketchFacts.SketchOrigin))
            {
                log.Warn("pushing " + locator + " although " + SketchObserver.Code + " applies (on_datum_plane="
                    + sketch.OnDatumPlane + ", measured_from=" + sketch.MeasuredFrom + ")");
            }
            var value = ToDimension(native, sign, unitMetres);
            var current = doc.GetValue(locator);
            if (current == null)
            {
                log.Warn("dimension not found: " + locator);
                status.Status = PushStatus.Failed;
                return status;
            }
            if (Math.Abs(current.Value - value) < 1e-12)
            {
                log.Debug("dimension " + locator + " already " + value + "; skipped");
                status.Status = PushStatus.Skipped;
                return status;
            }
            status.Status = doc.SetValue(locator, value) ? PushStatus.Pushed : PushStatus.Failed;
            log.Info("coordinate " + locator + " = " + value + " m (sign " + sign + "): " + status.Status);
            return status;
        }

        private static int CheckSign(int sign)
        {
            if (sign != 1 && sign != -1)
            {
                throw new ArgumentOutOfRangeException(nameof(sign), "sign must be 1 or -1");
            }
            return sign;
        }
    }
}

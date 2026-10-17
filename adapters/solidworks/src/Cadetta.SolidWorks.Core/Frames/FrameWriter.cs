using System;
using System.Collections.Generic;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Protocol;

namespace Cadetta.SolidWorks.Frames
{
    // frob:ticket 01M4SWMDXKYAFXR9GEKVTE7XA4
    /// <summary>Creates and updates coordinate systems from numeric values under datums = create (solidworks.md 4, adapters.md 3.4).</summary>
    public static class FrameWriter
    {
        /// <summary>
        /// Sets the coordinate system <paramref name="locator"/> to <paramref name="rows"/>
        /// (three rows of four, native unit): skipped when it already matches
        /// within the precision, created when missing, moved otherwise. A plane
        /// feature cannot be driven, so that locator fails.
        /// </summary>
        public static LocatorStatus Push(
            IFrameDocument doc, string locator, IReadOnlyList<double> rows, double unitMetres, int precision, Log log)
        {
            var status = new LocatorStatus { Locator = locator };
            var design = FrameTransform.FromRows(rows);
            var metres = design.Scaled(unitMetres);
            var existing = doc.ReadFrame(locator);
            if (existing != null)
            {
                if (existing.Kind == FrameKind.Plane)
                {
                    log.Warn("frame " + locator + " is a plane feature and cannot be driven");
                    status.Status = PushStatus.Failed;
                    return status;
                }
                var current = existing.Transform.Scaled(1.0 / unitMetres);
                if (current.Matches(design, precision, false))
                {
                    log.Debug("frame " + locator + " already matches; skipped");
                    status.Status = PushStatus.Skipped;
                    return status;
                }
                log.Info("frame " + locator + " moves from " + current + " to " + design);
            }
            else
            {
                log.Info("creating frame " + locator + " at " + design);
            }
            status.Status = doc.CreateOrUpdateCoordinateSystem(locator, metres) ? PushStatus.Pushed : PushStatus.Failed;
            if (status.Status == PushStatus.Failed)
            {
                log.Warn("frame " + locator + " was refused by SOLIDWORKS");
            }
            return status;
        }
    }
}

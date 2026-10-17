using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Push;

namespace Cadetta.SolidWorks.Commands
{
    // frob:ticket 01M4VB7VXXHJ6TKHF6J2STC1FE
    /// <summary>Link and unlink one dimension to a created global by the native equation `"D1@Feature" = "NAME"` (adapters.md 3.5); the name is given, never computed here.</summary>
    public static class LinkCommands
    {
        /// <summary>Links <paramref name="dimension"/> to <paramref name="global"/>, creating the global from the dimension's value when missing.</summary>
        public static CommandOutcome Link(IPushDocument doc, string dimension, string global, Log log)
        {
            var info = doc.GetDimension(dimension);
            if (info == null)
            {
                log.Warn("link: dimension " + dimension + " does not resolve");
                return CommandOutcome.Refused("dimension " + dimension + " does not resolve", "R010");
            }
            if (info.Driven || info.Suppressed || info.Conflict || info.ReadOnly)
            {
                log.Warn("link: dimension " + dimension + " cannot be driven");
                return CommandOutcome.Refused("dimension " + dimension + " is driven, suppressed, read-only or in conflict", "R010");
            }
            if (info.LinkedGlobal == global)
            {
                log.Info("link: " + dimension + " already linked to " + global);
                return CommandOutcome.Ok(dimension + " is already linked to " + global);
            }
            if (info.LinkedGlobal != null)
            {
                return CommandOutcome.Refused(dimension + " is linked to " + info.LinkedGlobal + "; unlink it first", "R010");
            }
            var created = false;
            if (doc.GetGlobal(global) == null)
            {
                if (!doc.SetGlobal(global, info.Value, info.Kind))
                {
                    return CommandOutcome.Refused("could not create global " + global, "R010");
                }
                created = true;
            }
            if (!doc.SetDimensionLink(dimension, global))
            {
                if (created)
                {
                    doc.DeleteGlobal(global);
                }
                log.Warn("link: SOLIDWORKS refused the equation for " + dimension);
                return CommandOutcome.Refused("SOLIDWORKS refused to link " + dimension, "R010");
            }
            log.Info("linked " + dimension + " to " + global + (created ? " (global created)" : ""));
            return CommandOutcome.Ok("linked " + dimension + " to " + global);
        }

        /// <summary>Removes the equation on <paramref name="dimension"/>, keeping its value.</summary>
        public static CommandOutcome Unlink(IPushDocument doc, string dimension, Log log)
        {
            var info = doc.GetDimension(dimension);
            if (info == null)
            {
                return CommandOutcome.Refused("dimension " + dimension + " does not resolve", "R010");
            }
            if (info.LinkedGlobal == null)
            {
                return CommandOutcome.Ok(dimension + " is not linked");
            }
            if (!doc.SetDimensionLink(dimension, null))
            {
                log.Warn("unlink: SOLIDWORKS refused for " + dimension);
                return CommandOutcome.Refused("SOLIDWORKS refused to unlink " + dimension, "R010");
            }
            log.Info("unlinked " + dimension);
            return CommandOutcome.Ok("unlinked " + dimension);
        }
    }
}

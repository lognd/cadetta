using Cadetta.SolidWorks.Guard;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Push;
using SolidWorks.Interop.sldworks;
using SolidWorks.Interop.swconst;

namespace Cadetta.SolidWorks.Com.Guard
{
    // frob:ticket 01M4SWMDS8MDHBS4T9JX1R3K6N
    /// <summary>Reads the facts <see cref="PushGuard"/> decides on (driven, suppressed, read-only, driving equation) from a SOLIDWORKS dimension.</summary>
    public static class SwComGuardFacts
    {
        /// <summary>Fills the guard fields of <paramref name="info"/> from <paramref name="d"/> and the right-hand side of its equation (null when it has none).</summary>
        public static void Fill(DimensionInfo info, IDimension d, string name, string? equationRhs, Log log)
        {
            info.Driven = d.DrivenState != (int)swDimensionDrivenState_e.swDimensionDriving;
            info.Suppressed = (d.GetFeatureOwner() as IFeature)?.IsSuppressed() ?? false;
            info.ReadOnly = d.ReadOnly;
            info.Conflict = false;
            info.Expression = null;
            info.LinkedGlobal = null;
            if (equationRhs != null)
            {
                // an equation-driven dimension reports the driven state, but it is still ours to push
                info.Driven = false;
                var target = PushGuard.LinkTarget(equationRhs);
                if (target != null)
                {
                    info.LinkedGlobal = target;
                }
                else
                {
                    info.Expression = equationRhs.Trim();
                }
            }
            log.Debug("guard facts " + name + ": driven=" + info.Driven + " suppressed=" + info.Suppressed + " readonly=" + info.ReadOnly + " expression=" + (info.Expression ?? "-") + " link=" + (info.LinkedGlobal ?? "-"));
        }
    }
}

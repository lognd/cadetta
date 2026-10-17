// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
using System;
using Cadetta.SolidWorks.AddIn.Pane;
using SolidWorks.Interop.sldworks;
using SolidWorks.Interop.swconst;

namespace Cadetta.SolidWorks.AddIn
{
    /// <summary>Reads the active document and its selected dimension from SOLIDWORKS (the only interop-touching read the pane needs).</summary>
    public static class SwSelection
    {
        /// <summary>Full path of the active document, or null when none is open or it is unsaved.</summary>
        public static string? ActiveDocumentPath(ISldWorks sw)
        {
            var doc = sw.ActiveDoc as ModelDoc2;
            var path = doc?.GetPathName();
            return string.IsNullOrEmpty(path) ? null : path;
        }

        /// <summary>The selected dimension's kind and SI value, or null when the selection is not a dimension.</summary>
        public static SelectedDimension? SelectedDimension(ISldWorks sw)
        {
            var doc = sw.ActiveDoc as ModelDoc2;
            var mgr = doc?.SelectionManager as ISelectionMgr;
            if (mgr == null || mgr.GetSelectedObjectCount2(-1) < 1)
            {
                return null;
            }
            if (mgr.GetSelectedObjectType3(1, -1) != (int)swSelectType_e.swSelDIMENSIONS)
            {
                return null;
            }
            var dim = (mgr.GetSelectedObject6(1, -1) as DisplayDimension)?.GetDimension2(0) as Dimension;
            if (dim == null)
            {
                return null;
            }
            var angular = ((IDimension)dim).GetType() == (int)swDimensionType_e.swAngularDimension;
            return new SelectedDimension(angular ? QuantityKind.Angle : QuantityKind.Length, dim.SystemValue);
        }
    }
}

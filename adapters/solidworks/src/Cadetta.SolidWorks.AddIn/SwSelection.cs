// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
using System;
using System.Collections.Generic;
using Cadetta.SolidWorks.Commands;
using Cadetta.SolidWorks.Frames;
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
            var angular = ((IDimension)dim).GetType() == (int)swDimensionParamType_e.swDimensionParamTypeDoubleAngular;
            return new SelectedDimension(angular ? QuantityKind.Angle : QuantityKind.Length, dim.SystemValue);
        }

        /// <summary>The selected dimension's document-local name such as "D1@Sketch1" (the form equations use), or null when the selection is not a dimension.</summary>
        public static string? SelectedDimensionName(ISldWorks sw)
        {
            var doc = sw.ActiveDoc as ModelDoc2;
            var mgr = doc?.SelectionManager as ISelectionMgr;
            if (mgr == null || mgr.GetSelectedObjectCount2(-1) < 1 || mgr.GetSelectedObjectType3(1, -1) != (int)swSelectType_e.swSelDIMENSIONS)
            {
                return null;
            }
            var dim = (mgr.GetSelectedObject6(1, -1) as DisplayDimension)?.GetDimension2(0) as Dimension;
            // Name is only "D1"; the equation form needs the owning feature, "D1@Boss-Extrude1" (FullName appends "@Doc.Part").
            var full = dim?.FullName;
            var cut = full?.LastIndexOf('@') ?? -1;
            return cut > 0 ? full!.Substring(0, cut) : full;
        }

        /// <summary>The selected assembly components with their poses in metres (empty when none are selected).</summary>
        // frob:ticket 01M54EDNCXH7QFEEGJQNAE1M1N
        public static List<SelectedInstance> SelectedComponents(ISldWorks sw)
        {
            var list = new List<SelectedInstance>();
            var mgr = (sw.ActiveDoc as ModelDoc2)?.SelectionManager as ISelectionMgr;
            if (mgr == null)
            {
                return list;
            }
            var count = mgr.GetSelectedObjectCount2(-1);
            for (var i = 1; i <= count; i++)
            {
                if (mgr.GetSelectedObjectType3(i, -1) != (int)swSelectType_e.swSelCOMPONENTS)
                {
                    continue;
                }
                var comp = mgr.GetSelectedObjectsComponent4(i, -1) as Component2;
                var array = comp?.Transform2?.ArrayData as double[];
                if (comp != null && array != null)
                {
                    list.Add(new SelectedInstance(comp.Name2, FrameTransform.FromSwArray(array)));
                }
            }
            return list;
        }
    }
}

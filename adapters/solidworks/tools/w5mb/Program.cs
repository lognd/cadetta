// Live driver for ~W5MBBTB: builds the r3-05 CoverPlate part on a real SOLIDWORKS
// (a C# net48 exe: PowerShell cannot cast the COM object to the interop ISldWorks).
//   w5mb build <part.SLDPRT> <log>    create the part, save it, close it (no doc left open)
//   w5mb edit  <part.SLDPRT> <log> <dim> <mm>   open, set one dimension, save, close
//   w5mb types <part.SLDPRT> <log>   list every dimension with its swDimensionType_e and system value
//   w5mb docs  <log>                  list the open documents
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using Cadetta.SolidWorks.Com.Session;
using Cadetta.SolidWorks.Logging;
using SolidWorks.Interop.sldworks;
using SolidWorks.Interop.swconst;

static class Program
{
    static StreamWriter o; static ISldWorks sw;
    static void L(string m) { o.WriteLine(m); o.Flush(); }
    static void Docs(string tag) { var n = new List<string>(); var d = (IModelDoc2)sw.GetFirstDocument(); while (d != null) { n.Add(d.GetTitle()); d = (IModelDoc2)d.GetNext(); } L("docs[" + tag + "] count=" + sw.GetDocumentCount() + " : " + string.Join(", ", n)); }
    static List<IFeature> Feats(IModelDoc2 doc) { var r = new List<IFeature>(); var f = (IFeature)doc.FirstFeature(); while (f != null) { r.Add(f); f = (IFeature)f.GetNextFeature(); } return r; }

    static int Main(string[] a)
    {
        o = new StreamWriter(a[a.Length - 1] == "docs" ? a[1] : a[2], true);
        try { InteropResolver.Install(new Log(TextWriter.Null, LogLevel.Error)); Run(a); }
        catch (Exception e) { L("EXCEPTION " + e); o.Close(); return 1; }
        o.Close(); return 0;
    }

    [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
    static void Run(string[] a)
    {
        sw = (ISldWorks)Marshal.GetActiveObject("SldWorks.Application");
        L("== w5mb " + a[0]);
        Docs("before");
        try
        {
            if (a[0] == "build") Build(a[1]);
            else if (a[0] == "types") Types(a[1]);
            else if (a[0] == "edit") Edit(a[1], a[3], double.Parse(a[4]));
        }
        finally { Docs("after"); }
    }

    static void Build(string path)
    {
        var tpl = sw.GetUserPreferenceStringValue((int)swUserPreferenceStringValue_e.swDefaultTemplatePart);
        var doc = (IModelDoc2)sw.NewDocument(tpl, 0, 0, 0);
        try
        {
            // Sketch1: 120 x 80 mm plate outline with D1 (length) and D2 (width).
            var plane = Feats(doc).First(f => f.GetTypeName2() == "RefPlane");
            doc.ClearSelection2(true); plane.Select2(false, 0);
            doc.SketchManager.InsertSketch(true);
            var segs = (object[])doc.SketchManager.CreateCornerRectangle(0, 0, 0, 0.12, 0.08, 0);
            foreach (object s in segs) { var seg = (ISketchSegment)s; L("rect seg type=" + seg.GetType()); }
            doc.ClearSelection2(true);
            ((ISketchSegment)segs[0]).Select4(false, null);
            L("dim length = " + (doc.AddDimension2(0.06, -0.02, 0) != null));
            doc.ClearSelection2(true);
            ((ISketchSegment)segs[1]).Select4(false, null);
            L("dim width = " + (doc.AddDimension2(0.14, 0.04, 0) != null));
            doc.ClearSelection2(true);
            doc.SketchManager.InsertSketch(true);
            doc.ClearSelection2(true);
            var sk = Feats(doc).Last(f => f.GetTypeName2() == "ProfileFeature"); sk.Select2(false, 0);
            // thickness 1/4 in = 6.35 mm.
            var plate = (IFeature)doc.FeatureManager.FeatureExtrusion2(true, false, false, 0, 0, 0.00635, 0, false, false, false, false, 0, 0, false, false, false, false, true, true, true, 0, 0, false);
            L("plate=" + plate.Name);

            // Sketch2: 12 mm pilot bore pad (diameter dimension D1@Sketch2), merged on the plate.
            doc.ClearSelection2(true);
            plane = Feats(doc).First(f => f.GetTypeName2() == "RefPlane"); plane.Select2(false, 0);
            doc.SketchManager.InsertSketch(true);
            var circ = (ISketchSegment)doc.SketchManager.CreateCircleByRadius(0.02, 0.02, 0, 0.006);
            doc.ClearSelection2(true); circ.Select4(false, null);
            L("dim diameter = " + (doc.AddDiameterDimension2(0.03, 0.03, 0) != null));
            doc.ClearSelection2(true);
            doc.SketchManager.InsertSketch(true);
            doc.ClearSelection2(true);
            sk = Feats(doc).Last(f => f.GetTypeName2() == "ProfileFeature"); sk.Select2(false, 0);
            var boss = (IFeature)doc.FeatureManager.FeatureExtrusion2(true, false, false, 0, 0, 0.010, 0, false, false, false, false, 0, 0, false, false, false, false, true, true, true, 0, 0, false);
            L("boss=" + boss.Name);

            // Linear pattern of the boss: count 4 (the bolt_count of the assembly).
            doc.ClearSelection2(true);
            boss.Select2(false, 4);
            var body = (IBody2)((object[])((IPartDoc)doc).GetBodies2(0, true))[0];
            var sd = ((ISelectionMgr)doc.SelectionManager).CreateSelectData(); sd.Mark = 1;
            bool picked = false;
            foreach (object eo in (object[])body.GetEdges()) { var e = (IEdge)eo; var cv = (ICurve)e.GetCurve(); if (cv.IsLine() && !picked) picked = ((IEntity)e).Select4(true, sd); }
            L("direction edge selected=" + picked);
            var pat = (IFeature)doc.FeatureManager.FeatureLinearPattern3(4, 0.015, 1, 0.01, false, false, "", "", false, false);
            if (pat == null) L("PATTERN NULL");
            else
            {
                L("pattern=" + pat.Name + " type=" + pat.GetTypeName2());
                var dd = (IDisplayDimension)pat.GetFirstDisplayDimension();
                while (dd != null) { var dm = dd.GetDimension2(0); L("pattern dim: " + dm.FullName + " = " + dm.SystemValue); dd = (IDisplayDimension)pat.GetNextDisplayDimension(dd); }
            }
            foreach (var f in Feats(doc))
            {
                var dd = (IDisplayDimension)f.GetFirstDisplayDimension();
                while (dd != null) { var dm = dd.GetDimension2(0); L("dim " + dm.FullName + " = " + dm.SystemValue); dd = (IDisplayDimension)f.GetNextDisplayDimension(dd); }
            }
            Directory.CreateDirectory(Path.GetDirectoryName(path));
            if (File.Exists(path)) File.Delete(path);
            int e1 = 0, w1 = 0;
            L("saved=" + doc.Extension.SaveAs3(path, 0, 1, null, null, ref e1, ref w1) + " err=" + e1);
        }
        finally { sw.CloseDoc(doc.GetTitle()); }
    }

    static void Types(string path)
    {
        int e = 0, w = 0;
        var doc = sw.OpenDoc6(path, (int)swDocumentTypes_e.swDocPART, (int)swOpenDocOptions_e.swOpenDocOptions_Silent, "", ref e, ref w);
        try
        {
            foreach (var f in Feats(doc))
            {
                var dd = (IDisplayDimension)f.GetFirstDisplayDimension();
                while (dd != null)
                {
                    var dm = dd.GetDimension2(0);
                    L("dim " + dm.FullName + " swDimensionType=" + ((IDimension)dm).GetType() + " system=" + dm.SystemValue);
                    dd = (IDisplayDimension)f.GetNextDisplayDimension(dd);
                }
            }
        }
        finally { sw.CloseDoc(doc.GetTitle()); }
    }

    static void Edit(string path, string dim, double mm)
    {
        int e = 0, w = 0;
        var doc = sw.OpenDoc6(path, (int)swDocumentTypes_e.swDocPART, (int)swOpenDocOptions_e.swOpenDocOptions_Silent, "", ref e, ref w);
        try
        {
            var d = (IDimension)doc.Parameter(dim);
            L("before " + dim + " = " + d.SystemValue);
            L("set rc=" + d.SetSystemValue3(mm / 1000.0, (int)swSetValueInConfiguration_e.swSetValue_InThisConfiguration, null));
            doc.ForceRebuild3(false);
            L("after " + dim + " = " + ((IDimension)doc.Parameter(dim)).SystemValue);
            int e1 = 0, w1 = 0;
            L("saved=" + doc.Extension.SaveAs3(path, 0, 1, null, null, ref e1, ref w1));
        }
        finally { sw.CloseDoc(doc.GetTitle()); }
    }
}

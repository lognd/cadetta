using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json;
using Cadetta.SolidWorks.Com.Push;
using Cadetta.SolidWorks.Com.Session;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Protocol;
using Cadetta.SolidWorks.Push;
using SolidWorks.Interop.sldworks;
using SolidWorks.Interop.swconst;

static class Program
{
    static StreamWriter o; static int fail; static ISldWorks sw; static Log log; static PushService svc;
    static void L(string m) { o.WriteLine(m); o.Flush(); }
    static void Check(string n, bool c, string d) { if (c) L("PASS " + n + " :: " + d); else { L("FAIL " + n + " :: " + d); fail++; } }
    static void Docs(string tag) { var n = new List<string>(); var d = (IModelDoc2)sw.GetFirstDocument(); while (d != null) { n.Add(d.GetTitle()); d = (IModelDoc2)d.GetNext(); } L("docs[" + tag + "] count=" + sw.GetDocumentCount() + " : " + string.Join(", ", n)); }
    static List<IFeature> Feats(IModelDoc2 doc) { var r = new List<IFeature>(); var f = (IFeature)doc.FirstFeature(); while (f != null) { r.Add(f); f = (IFeature)f.GetNextFeature(); } return r; }
    static IFeature Boss(IModelDoc2 doc, double depth, bool rect)
    {
        var plane = Feats(doc).First(f => f.GetTypeName2() == "RefPlane");
        doc.ClearSelection2(true); plane.Select2(false, 0);
        doc.SketchManager.InsertSketch(true);
        if (rect) doc.SketchManager.CreateCornerRectangle(0, 0, 0, 0.04, 0.03, 0); else doc.SketchManager.CreateCircleByRadius(0.005, 0.005, 0, 0.002);
        doc.SketchManager.InsertSketch(true);
        doc.ClearSelection2(true);
        var sk = Feats(doc).Last(f => f.GetTypeName2() == "ProfileFeature");
        sk.Select2(false, 0);
        return (IFeature)doc.FeatureManager.FeatureExtrusion2(true, false, false, 0, 0, depth, 0, false, false, false, false, 0, 0, false, false, false, false, true, true, true, 0, 0, false);
    }
    static ChangedParam CP(string loc, string mode, string path, string val, string refTo = null)
    {
        var c = new ChangedParam { Locator = loc, Mode = mode, Path = path, Value = JsonDocument.Parse(val).RootElement.Clone() };
        if (refTo != null) c.RefTo = refTo; return c;
    }
    static PushOutcome Push(SwComPushDocument pd, params ChangedParam[] cs) => svc.Push(pd, cs.ToList(), new PushOptions());
    static string Sts(PushOutcome o) => string.Join("; ", o.Statuses.Select(s => s.Locator + "=" + s.Status));
    static string Codes(PushOutcome o) => string.Join(",", o.Diagnostics.Select(d => d.GetProperty("code").GetString()));
    static string Msgs(PushOutcome o) => string.Join(" | ", o.Diagnostics.Select(d => d.GetProperty("message").GetString()));

    static int Main(string[] a)
    {
        o = new StreamWriter(@"C:\src\pf7x-live.log", false);
        try { log = new Log(TextWriter.Null, LogLevel.Error); InteropResolver.Install(log); Run(); }
        catch (Exception e) { L("EXCEPTION " + e); fail++; }
        L("FAILURES: " + fail); o.Close(); return fail;
    }
    [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
    static void Run()
    {
        sw = (ISldWorks)Marshal.GetActiveObject("SldWorks.Application");
        svc = new PushService(log);
        Docs("before");
        var tpl = sw.GetUserPreferenceStringValue((int)swUserPreferenceStringValue_e.swDefaultTemplatePart);
        Directory.CreateDirectory(@"C:\src\pf7x");
        int e1 = 0, w1 = 0;

        // part 1: plate + boss + linear pattern
        var doc = (IModelDoc2)sw.NewDocument(tpl, 0, 0, 0);
        try
        {
            var plate = Boss(doc, 0.006, true);
            var boss = Boss(doc, 0.010, false);
            L("plate=" + plate.Name + " boss=" + boss.Name);
            doc.ClearSelection2(true);
            boss.Select2(false, 4);
            var body = (IBody2)((object[])((IPartDoc)doc).GetBodies2(0, true))[0];
            var sd = ((ISelectionMgr)doc.SelectionManager).CreateSelectData(); sd.Mark = 1;
            bool picked = false;
            foreach (object eo in (object[])body.GetEdges()) { var e = (IEdge)eo; var cv = (ICurve)e.GetCurve(); if (cv.IsLine() && !picked) picked = ((IEntity)e).Select4(true, sd); }
            L("direction edge selected=" + picked);
            var pat = (IFeature)doc.FeatureManager.FeatureLinearPattern3(3, 0.012, 1, 0.01, false, false, "", "", false, false);
            string cdim = null;
            if (pat == null) { L("pattern creation returned null"); fail++; }
            else
            {
                L("pattern=" + pat.Name + " type=" + pat.GetTypeName2());
                var dd = (IDisplayDimension)pat.GetFirstDisplayDimension();
                while (dd != null) { var dm = dd.GetDimension2(0); L("pattern dim: " + dm.FullName + " = " + dm.SystemValue); if (Math.Abs(dm.SystemValue - 3) < 1e-9) cdim = dm.FullName.Substring(0, dm.FullName.IndexOf("@", dm.FullName.IndexOf("@") + 1)); dd = (IDisplayDimension)pat.GetNextDisplayDimension(dd); }
            }
            doc.Extension.SaveAs3(@"C:\src\pf7x\Cover.SLDPRT", 0, 1, null, null, ref e1, ref w1);
            L("saved Cover, dirty=" + doc.GetSaveFlag());
            var pd = new SwComPushDocument(doc, "Cover.SLDPRT", log);
            if (cdim != null)
            {
                var r = Push(pd, CP(cdim, "ref", "cover.holes.count", "\"5\"", "HoleCount"));
                L("count push: " + Sts(r) + " codes=" + Codes(r) + " msgs=" + Msgs(r));
                var di = pd.GetDimension(cdim);
                Check("pattern count linked", di.LinkedGlobal == "HoleCount", "linked=" + di.LinkedGlobal + " value=" + di.Value + " HoleCount=" + pd.GetGlobal("HoleCount"));
                Check("pattern count value 5", Math.Abs(di.Value - 5) < 1e-9, "value=" + di.Value);
                r = Push(pd, CP(cdim, "ref", "cover.holes.count", "\"5\"", "HoleCount"));
                Check("pattern count unchanged skipped", Sts(r) == cdim + "=skipped", Sts(r));
                r = Push(pd, CP(cdim, "ref", "cover.holes.count", "\"4\"", "HoleCount"));
                Check("pattern count re-push", pd.GetDimension(cdim).Value == 4, Sts(r) + " value=" + pd.GetDimension(cdim).Value);
            }
            var tdim = "D1@" + plate.Name;
            foreach (var t in new[] { ("bilateral", "Bilateral"), ("fit", "Fit") })
            {
                var c = CP(tdim, "value", "cover.thickness", "\"6\"");
                var tol = new DimTolerance { Form = t.Item1 };
                if (t.Item1 == "bilateral") { tol.Lower = "-0.1"; tol.Upper = "0.2"; } else tol.Class = "H7";
                c.Tolerance = tol;
                var r = Push(pd, c);
                var di = pd.GetDimension(tdim);
                Check("tolerance " + t.Item1, di.Tolerance.Kind.ToString() == t.Item2, "kind=" + di.Tolerance.Kind + " lo=" + di.Tolerance.Lower + " up=" + di.Tolerance.Upper + " " + Sts(r) + " " + Codes(r));
            }
        }
        finally { sw.CloseDoc(doc.GetTitle()); Docs("after part1"); }

        // part 2: rebuild-failure restore
        doc = (IModelDoc2)sw.NewDocument(tpl, 0, 0, 0);
        try
        {
            Boss(doc, 0.006, true);
            var b = (IBody2)((object[])((IPartDoc)doc).GetBodies2(0, true))[0];
            var face = (IFace2)b.GetFirstFace(); doc.ClearSelection2(true); ((IEntity)face).Select4(false, null);
            doc.InsertFeatureShell(0.001, false);
            var shell = Feats(doc).Last(f => f.GetTypeName2() == "Shell");
            var sdim = "D1@" + shell.Name; L("shell dim " + sdim);
            var eq = doc.GetEquationMgr();
            eq.Add2(-1, "\"PROBE_S\" = 1mm", true);
            eq.Add2(-1, "\"" + sdim + "\" = \"PROBE_S\"", true);
            doc.ForceRebuild3(false);
            doc.Extension.SaveAs3(@"C:\src\pf7x\Shelled.SLDPRT", 0, 1, null, null, ref e1, ref w1);
            var pd = new SwComPushDocument(doc, "Shelled.SLDPRT", log);
            var before = pd.GetGlobal("PROBE_S");
            var r = Push(pd, CP(sdim, "ref", "shelled.wall", "\"50\"", "PROBE_S"));
            L("failing push: " + Sts(r) + " codes=" + Codes(r) + " msgs=" + Msgs(r));
            var after = pd.GetGlobal("PROBE_S");
            Check("R004 names feature", Codes(r).Contains("R004") && Msgs(r).Contains(shell.Name), "codes=" + Codes(r));
            Check("global restored after failed rebuild", Math.Abs((double)after - (double)before) < 1e-9 && Math.Abs((double)before - 1) < 1e-9, "before=" + before + " after=" + after);
            var d2 = pd.GetDimension(sdim);
            Check("shell dim restored", Math.Abs(d2.Value - 1) < 1e-6 && d2.LinkedGlobal == "PROBE_S", "value=" + d2.Value + " linked=" + d2.LinkedGlobal);
            Check("rebuild clean after restore", doc.ForceRebuild3(false), "dirty=" + doc.GetSaveFlag());
        }
        finally { sw.CloseDoc(doc.GetTitle()); Docs("after part2"); }
        Docs("final");
    }
}

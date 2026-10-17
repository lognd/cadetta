using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Probe;
using SolidWorks.Interop.sldworks;
using SolidWorks.Interop.swconst;

namespace Cadetta.SolidWorks.Com.Probe
{
    // frob:ticket ~Z9SWC8W
    /// <summary>
    /// ISwApiProbe over COM: builds a small part and assembly in a scratch
    /// directory and exercises each API solidworks.md 4 expects, recording the
    /// call used and what came back (adapters/solidworks/API-SURFACE.md).
    /// </summary>
    public sealed class SwComApiProbe : ISwApiProbe
    {
        private string Dim = "D1@Boss-Extrude1"; // set from the real feature name in BuildPart
        private readonly ISldWorks _sw;
        private readonly Log _log;
        private readonly List<ApiProbeResult> _results = new List<ApiProbeResult>();
        private ModelDoc2? _part;
        private string _partPath = "";
        private string _scratch = "";

        /// <summary>Probes the application <paramref name="sw"/>, logging to <paramref name="log"/>.</summary>
        public SwComApiProbe(ISldWorks sw, Log log)
        {
            _sw = sw;
            _log = log;
        }

        /// <summary>Runs every probe in <paramref name="scratchDir"/> and returns the results in order.</summary>
        public IReadOnlyList<ApiProbeResult> Run(string scratchDir)
        {
            _results.Clear();
            _scratch = scratchDir;
            Directory.CreateDirectory(scratchDir);
            _log.Info("api probe in " + scratchDir);

            Step("session", "ISldWorks.RevisionNumber()", () => _sw.RevisionNumber());
            Step("session", "ISldWorks.GetUserPreferenceStringValue(swDefaultTemplatePart)", () =>
                Require(_sw.GetUserPreferenceStringValue((int)swUserPreferenceStringValue_e.swDefaultTemplatePart)));

            if (Step("part", "ISldWorks.NewDocument + SketchManager.CreateCornerRectangle + FeatureManager.FeatureExtrusion2",
                () => BuildPart(scratchDir)))
            {
                ProbeEquations();
                ProbeDimension();
                ProbeTolerance();
                ProbeProperties();
                ProbeCoordinateSystem();
                ProbeRebuild();
                var part = _part!;
                Step("part", "IModelDocExtension.SaveAs3", () => SavePart(part));
                ProbeComponent();
                Step("part", "ISldWorks.CloseDoc", () =>
                {
                    _sw.CloseDoc(part.GetTitle());
                    return "closed";
                });
            }
            return _results;
        }

        private bool Step(string api, string call, Func<string> body)
        {
            try
            {
                var detail = body();
                _log.Info("probe ok " + api + " " + call + " -> " + detail);
                _results.Add(new ApiProbeResult(api, call, true, detail));
                return true;
            }
            catch (Exception e)
            {
                _log.Warn("probe FAIL " + api + " " + call + ": " + e.Message);
                _results.Add(new ApiProbeResult(api, call, false, e.GetType().Name + ": " + e.Message));
                return false;
            }
        }

        private static string Require(string? s)
        {
            if (string.IsNullOrEmpty(s))
            {
                throw new InvalidOperationException("empty result");
            }
            return s!;
        }

        private static void Expect(bool cond, string what)
        {
            if (!cond)
            {
                throw new InvalidOperationException("expectation failed: " + what);
            }
        }

        private static string F(double v) => v.ToString("R", CultureInfo.InvariantCulture);

        private static Feature? FindFeature(ModelDoc2 doc, string typeName, bool last)
        {
            Feature? found = null;
            for (var f = (Feature?)doc.FirstFeature(); f != null; f = (Feature?)f.GetNextFeature())
            {
                if (f.GetTypeName2() == typeName)
                {
                    found = f;
                    if (!last)
                    {
                        break;
                    }
                }
            }
            return found;
        }

        private string BuildPart(string scratchDir)
        {
            var template = _sw.GetUserPreferenceStringValue((int)swUserPreferenceStringValue_e.swDefaultTemplatePart);
            var doc = (ModelDoc2?)_sw.NewDocument(template, 0, 0, 0)
                ?? throw new InvalidOperationException("NewDocument returned null for " + template);
            _part = doc;
            // Names are template/locale dependent: find features by type, not by name.
            var plane = FindFeature(doc, "RefPlane", false) ?? throw new InvalidOperationException("no RefPlane feature in template");
            Expect(plane.Select2(false, 0), "select plane " + plane.Name);
            doc.SketchManager.InsertSketch(true);
            var segs = doc.SketchManager.CreateCornerRectangle(0, 0, 0, 0.04, 0.03, 0);
            Expect(segs != null, "CreateCornerRectangle returned null (no active sketch?)");
            doc.SketchManager.InsertSketch(true);
            doc.ClearSelection2(true);
            var sketch = FindFeature(doc, "ProfileFeature", true) ?? throw new InvalidOperationException("no sketch feature after InsertSketch");
            Expect(sketch.Select2(false, 0), "select sketch " + sketch.Name);
            var feat = doc.FeatureManager.FeatureExtrusion2(true, false, false, 0, 0, 0.006, 0, false, false, false, false,
                0, 0, false, false, false, false, true, true, true, 0, 0, false)
                ?? throw new InvalidOperationException("FeatureExtrusion2 returned null after selecting " + sketch.Name);
            Dim = "D1@" + feat.Name;
            return "feature " + feat.Name + " (" + feat.GetTypeName2() + ") in " + template;
        }

        private void ProbeEquations()
        {
            var doc = _part!;
            var eq = doc.GetEquationMgr();
            Step("equation manager", "IModelDoc2.GetEquationMgr().Add2(-1, \"\\\"PROBE_T\\\" = 6mm\", true)", () =>
            {
                var i = eq.Add2(-1, "\"PROBE_T\" = 6mm", true);
                Expect(i >= 0, "index " + i);
                Expect(eq.get_GlobalVariable(i), "GlobalVariable(" + i + ")");
                return "index " + i + ", Equation=" + eq.get_Equation(i) + ", Value=" + F(eq.get_Value(i)) + ", GlobalVariable=true";
            });
            Step("equation manager", "IEquationMgr.Add2(-1, \"\\\"" + Dim + "\\\" = \\\"PROBE_T\\\"\", true) (ref mode)", () =>
            {
                var i = eq.Add2(-1, "\"" + Dim + "\" = \"PROBE_T\"", true);
                Expect(i >= 0, "index " + i);
                doc.ForceRebuild3(false);
                var v = ((Dimension)doc.Parameter(Dim)).SystemValue;
                Expect(Math.Abs(v - 0.006) < 1e-9, "dimension follows global, got " + F(v));
                return "index " + i + ", Equation=" + eq.get_Equation(i) + ", dimension=" + F(v) + " m, Status=" + eq.Status;
            });
            Step("equation manager", "IEquationMgr.set_Equation(0, \"\\\"PROBE_T\\\" = 8mm\") then ForceRebuild3", () =>
            {
                eq.set_Equation(0, "\"PROBE_T\" = 8mm");
                doc.ForceRebuild3(false);
                var v = ((Dimension)doc.Parameter(Dim)).SystemValue;
                Expect(Math.Abs(v - 0.008) < 1e-9, "dimension follows edited global, got " + F(v));
                return "dimension=" + F(v) + " m, GetCount=" + eq.GetCount();
            });
        }

        private void ProbeDimension()
        {
            var doc = _part!;
            Step("IDimension", "IModelDoc2.Parameter(\"" + Dim + "\") as Dimension; Name, FullName, SystemValue, DrivenState", () =>
            {
                var d = (Dimension)doc.Parameter(Dim);
                return "Name=" + d.Name + ", FullName=" + d.FullName + ", SystemValue=" + F(d.SystemValue) + " m, DrivenState=" + d.DrivenState;
            });
        }

        private void ProbeTolerance()
        {
            var doc = _part!;
            var tol = ((Dimension)doc.Parameter(Dim)).Tolerance;
            string Read() => "Type=" + tol.Type + ", min=" + F(tol.GetMinValue()) + ", max=" + F(tol.GetMaxValue());
            Step("IDimensionTolerance", "Tolerance.Type = swTolSYMMETRIC; SetValues(0.0001, 0.0001)", () =>
            {
                tol.Type = (int)swTolType_e.swTolSYMMETRIC;
                Expect(tol.SetValues(0.0001, 0.0001), "SetValues returned false");
                return Read();
            });
            Step("IDimensionTolerance", "Tolerance.Type = swTolBILAT; SetValues(-0.0001, 0.0002)", () =>
            {
                tol.Type = (int)swTolType_e.swTolBILAT;
                Expect(tol.SetValues(-0.0001, 0.0002), "SetValues returned false");
                return Read();
            });
            Step("IDimensionTolerance", "Tolerance.Type = swTolLIMIT; SetValues(-0.0001, 0.0002)", () =>
            {
                tol.Type = (int)swTolType_e.swTolLIMIT;
                Expect(tol.SetValues(-0.0001, 0.0002), "SetValues returned false");
                return Read();
            });
            Step("IDimensionTolerance", "Tolerance.Type = swTolFIT; FitType = swFitCLEARANCE; SetFitValues(\"H7\", \"g6\")", () =>
            {
                tol.Type = (int)swTolType_e.swTolFIT;
                tol.FitType = (int)swFitType_e.swFitCLEARANCE;
                Expect(tol.SetFitValues("H7", "g6"), "SetFitValues returned false");
                return Read() + ", FitType=" + tol.FitType + ", hole=" + tol.GetHoleFitValue() + ", shaft=" + tol.GetShaftFitValue();
            });
            Step("IDimensionTolerance", "Tolerance.Type = swTolNONE", () =>
            {
                tol.Type = (int)swTolType_e.swTolNONE;
                return Read();
            });
        }

        private void ProbeProperties()
        {
            var doc = _part!;
            Step("custom properties", "Extension.CustomPropertyManager[\"\"].Add3 then Get5", () =>
            {
                var mgr = doc.Extension.CustomPropertyManager[""];
                var r = mgr.Add3("PROBE_PROP", (int)swCustomInfoType_e.swCustomInfoText, "v1",
                    (int)swCustomPropertyAddOption_e.swCustomPropertyReplaceValue);
                string val, resolved;
                bool was;
                mgr.Get5("PROBE_PROP", false, out val, out resolved, out was);
                Expect(val == "v1", "value " + val);
                return "Add3=" + r + ", Get5 value=" + val + ", resolved=" + resolved + ", count=" + mgr.Count;
            });
            Step("custom properties", "Extension.CustomPropertyManager[configName].Add3 then Get5 (configuration-specific)", () =>
            {
                var cfg = doc.ConfigurationManager.ActiveConfiguration.Name;
                var mgr = doc.Extension.CustomPropertyManager[cfg];
                mgr.Add3("PROBE_CFG", (int)swCustomInfoType_e.swCustomInfoText, "c1",
                    (int)swCustomPropertyAddOption_e.swCustomPropertyReplaceValue);
                string val, resolved;
                bool was;
                mgr.Get5("PROBE_CFG", false, out val, out resolved, out was);
                var general = doc.Extension.CustomPropertyManager[""].Get("PROBE_CFG");
                Expect(val == "c1", "value " + val);
                Expect(string.IsNullOrEmpty(general), "not visible at file level, saw " + general);
                return "config=" + cfg + ", value=" + val + ", absent at file level";
            });
        }

        private void ProbeCoordinateSystem()
        {
            var doc = _part!;
            Step("coordinate system", "IFeatureManager.CreateCoordinateSystemUsingNumericalValues(true, 0.01, 0.02, 0.03, true, 0, 0, 0)", () =>
            {
                var f = doc.FeatureManager.CreateCoordinateSystemUsingNumericalValues(true, 0.01, 0.02, 0.03, true, 0, 0, 0)
                    ?? throw new InvalidOperationException("returned null");
                f.Name = "PROBE_CS";
                return "feature " + f.Name + " type " + f.GetTypeName2();
            });
            Step("coordinate system", "IModelDocExtension.GetCoordinateSystemTransformByName(\"PROBE_CS\").ArrayData", () =>
            {
                var t = doc.Extension.GetCoordinateSystemTransformByName("PROBE_CS")
                    ?? throw new InvalidOperationException("returned null");
                var a = ((double[])t.ArrayData);
                Expect(Math.Abs(a[9] - 0.01) < 1e-9 && Math.Abs(a[10] - 0.02) < 1e-9 && Math.Abs(a[11] - 0.03) < 1e-9,
                    "translation " + string.Join(",", a.Skip(9).Take(3).Select(F)));
                return "ArrayData=[" + string.Join(", ", a.Select(F)) + "]";
            });
            Step("coordinate system", "CreateCoordinateSystemUsingNumericalValues(true, 0, 0, 0, true, 0, 0, 1.5707963267949) (rotation convention)", () =>
            {
                var f = doc.FeatureManager.CreateCoordinateSystemUsingNumericalValues(true, 0, 0, 0, true, 0, 0, Math.PI / 2)
                    ?? throw new InvalidOperationException("returned null");
                f.Name = "PROBE_CS_RZ";
                var a = (double[])doc.Extension.GetCoordinateSystemTransformByName("PROBE_CS_RZ").ArrayData;
                return "rotate 90deg about z: ArrayData=[" + string.Join(", ", a.Select(F)) + "]";
            });
        }

        private void ProbeRebuild()
        {
            var doc = _part!;
            Step("rebuild", "IModelDoc2.ForceRebuild3(false) returns true on a healthy model", () =>
                "ForceRebuild3=" + doc.ForceRebuild3(false) + ", errors=" + RebuildErrors(doc).Count);
            Step("rebuild", "IModelDoc2.InsertFeatureShell(0.001, false); IEquationMgr.set_Equation(shell dim = 50mm); ForceRebuild3; walk IFeature.GetErrorCode2", () =>
            {
                // Equations refuse non-positive depths silently, so use a shell too thick for the part.
                var part = (PartDoc)doc;
                var body = ((object[])part.GetBodies2((int)swBodyType_e.swSolidBody, true))[0] as Body2
                    ?? throw new InvalidOperationException("no solid body");
                var face = (Face2)body.GetFirstFace();
                doc.ClearSelection2(true);
                Expect(((Entity)face).Select4(false, null), "select face");
                doc.InsertFeatureShell(0.001, false);
                var shell = FindFeature(doc, "Shell", true) ?? throw new InvalidOperationException("no Shell feature");
                var sdim = "D1@" + shell.Name;
                var eq = doc.GetEquationMgr();
                var gi = eq.Add2(-1, "\"PROBE_S\" = 1mm", true);
                var ei = eq.Add2(-1, "\"" + sdim + "\" = \"PROBE_S\"", true);
                doc.ForceRebuild3(false);
                var healthy = RebuildErrors(doc).Count;
                eq.set_Equation(gi, "\"PROBE_S\" = 50mm");
                var ret = doc.ForceRebuild3(false);
                var errs = RebuildErrors(doc);
                var dim = ((Dimension)doc.Parameter(sdim)).SystemValue;
                eq.set_Equation(gi, "\"PROBE_S\" = 1mm");
                doc.ForceRebuild3(false);
                var after = RebuildErrors(doc).Count;
                var detail = "shell=" + shell.Name + ", healthy errors=" + healthy + ", bad: ForceRebuild3=" + ret + ", dim=" + F(dim) + ", errors=[" + string.Join("; ", errs) + "], after restore " + after;
                Expect(errs.Count > 0, "a failing rebuild lists an errored feature; " + detail);
                Expect(after == 0, "restore clears errors; " + detail);
                return detail;
            });
        }

        private static List<string> RebuildErrors(ModelDoc2 doc)
        {
            var errs = new List<string>();
            var f = (Feature?)doc.FirstFeature();
            while (f != null)
            {
                bool warn;
                var code = f.GetErrorCode2(out warn);
                if (code != 0)
                {
                    errs.Add(f.Name + " code " + code + (warn ? " (warning)" : ""));
                }
                f = (Feature?)f.GetNextFeature();
            }
            return errs;
        }

        private string SavePart(ModelDoc2 part)
        {
            _partPath = Path.Combine(_scratch, "Probe.SLDPRT");
            int err = 0, warn = 0;
            var ok = part.Extension.SaveAs3(_partPath, 0, 1, null, null, ref err, ref warn);
            Expect(ok && File.Exists(_partPath), "saved " + _partPath + " errors=" + err);
            return "SaveAs3 ok, errors=" + err + ", warnings=" + warn;
        }

        private void ProbeComponent()
        {
            if (!File.Exists(_partPath))
            {
                return;
            }
            ModelDoc2? asm = null;
            IComponent2? comp = null;
            if (!Step("component", "ISldWorks.NewDocument(assembly template); IAssemblyDoc.AddComponent5(path, 0, \"\", false, \"\", 0, 0, 0)", () =>
            {
                var template = _sw.GetUserPreferenceStringValue((int)swUserPreferenceStringValue_e.swDefaultTemplateAssembly);
                asm = (ModelDoc2)_sw.NewDocument(template, 0, 0, 0);
                comp = ((AssemblyDoc)asm).AddComponent5(_partPath, 0, "", false, "", 0, 0, 0)
                    ?? throw new InvalidOperationException("AddComponent5 returned null");
                return "Name2=" + comp.Name2 + ", PathName=" + comp.GetPathName();
            }))
            {
                return;
            }
            var asmDoc = asm!;
            var c = comp!;
            Step("component", "IComponent2.Transform2 (get) .ArrayData", () =>
                "ArrayData=[" + string.Join(", ", ((double[])c.Transform2.ArrayData).Select(F)) + "]");
            Step("component", "IMathUtility.CreateTransform(double[16]); IComponent2.Transform2 = t; read back", () =>
            {
                var mu = (MathUtility)_sw.GetMathUtility();
                var data = new double[] { 1, 0, 0, 0, 1, 0, 0, 0, 1, 0.05, 0.06, 0.07, 1, 0, 0, 0 };
                c.Transform2 = (MathTransform)mu.CreateTransform(data);
                asmDoc.ForceRebuild3(false);
                var a = (double[])c.Transform2.ArrayData;
                Expect(Math.Abs(a[9] - 0.05) < 1e-9 && Math.Abs(a[10] - 0.06) < 1e-9 && Math.Abs(a[11] - 0.07) < 1e-9,
                    "translation " + string.Join(",", a.Skip(9).Take(3).Select(F)));
                return "ArrayData=[" + string.Join(", ", a.Select(F)) + "]";
            });
            Step("component", "IComponent2.ComponentReference set and get", () =>
            {
                c.ComponentReference = "PROBE_REF";
                var back = c.ComponentReference;
                Expect(back == "PROBE_REF", "read back " + back);
                return "ComponentReference=" + back;
            });
            Step("rebuild", "assembly IModelDoc2.ForceRebuild3(false)", () => "ForceRebuild3=" + asmDoc.ForceRebuild3(false));
            Step("component", "ISldWorks.CloseDoc (assembly, unsaved)", () =>
            {
                _sw.CloseDoc(asmDoc.GetTitle());
                return "closed";
            });
        }
    }
}

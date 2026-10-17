using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text.RegularExpressions;
using Cadetta.SolidWorks.Com.Assembly;
using Cadetta.SolidWorks.Com.Props;
using Cadetta.SolidWorks.Props;
using Cadetta.SolidWorks.Push;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Read;
using SolidWorks.Interop.sldworks;
using SolidWorks.Interop.swconst;

namespace Cadetta.SolidWorks.Com.Read
{
    /// <summary>
    /// INativeDocumentReader over SOLIDWORKS COM: opens a file read-only and
    /// extracts globals, dimensions, tolerances, properties, coordinate
    /// systems and component transforms (solidworks.md 4).
    /// frob:ticket 01M4SV2TKZD26CK9FFA1EJXSV4
    /// </summary>
    public sealed class SwComDocumentReader : INativeDocumentReader
    {
        private static readonly Regex Plain = new Regex(@"^\s*[-+]?[0-9.]+(?:[eE][-+]?\d+)?\s*[a-zA-Z]*\s*$", RegexOptions.Compiled);
        private static readonly Regex EqLeft = new Regex("^\\s*\"([^\"]+)\"\\s*=\\s*(.*)$", RegexOptions.Compiled | RegexOptions.Singleline);

        private readonly ISldWorks _sw;
        private readonly Log _log;

        /// <summary>Creates a reader over the running SOLIDWORKS <paramref name="sw"/>.</summary>
        public SwComDocumentReader(ISldWorks sw, Log log)
        {
            _sw = sw;
            _log = log;
        }

        /// <summary>Metres to a millimetre decimal literal, rounded to 9 places to drop binary noise.</summary>
        public static string Mm(double metres) => Num(metres * 1000.0, 9);

        /// <summary>A plain number literal rounded to <paramref name="places"/> decimals (no negative zero).</summary>
        public static string Num(double v, int places)
        {
            return DimensionUnits.Literal(v, places);
        }

        /// <summary>Opens <paramref name="path"/> read-only (reusing an open copy) and extracts its native model.</summary>
        public NativeDocument Read(string path)
        {
            var doc = new NativeDocument { Path = path };
            var model = _sw.GetOpenDocumentByName(path) as ModelDoc2;
            var opened = false;
            if (model == null)
            {
                int errors = 0, warnings = 0;
                var type = TypeOf(path);
                model = _sw.OpenDoc6(path, type,
                    (int)swOpenDocOptions_e.swOpenDocOptions_Silent | (int)swOpenDocOptions_e.swOpenDocOptions_ReadOnly,
                    "", ref errors, ref warnings);
                if (model == null)
                {
                    throw new InvalidOperationException("OpenDoc6 failed for " + path + ": errors=" + errors);
                }
                opened = true;
                _log.Info("opened " + path + " (warnings=" + warnings + ")");
            }
            try
            {
                ReadEquations(model, doc);
                ReadDimensions(model, doc);
                ReadProperties(model, doc);
                ReadFrames(model, doc);
                if (model is AssemblyDoc asm)
                {
                    ReadInstances(asm, doc);
                    SwComComponentReader.ReadDimensions(this, asm, doc, _log);
                }
            }
            finally
            {
                if (opened)
                {
                    _sw.CloseDoc(model.GetTitle());
                    _log.Debug("closed " + path);
                }
            }
            return doc;
        }

        private static int TypeOf(string path)
        {
            switch (Path.GetExtension(path).ToLowerInvariant())
            {
                case ".sldasm": return (int)swDocumentTypes_e.swDocASSEMBLY;
                case ".slddrw": return (int)swDocumentTypes_e.swDocDRAWING;
                default: return (int)swDocumentTypes_e.swDocPART;
            }
        }

        private readonly Dictionary<string, string> _dimEquations = new Dictionary<string, string>(StringComparer.Ordinal);

        /// <summary>The document length unit as a unit string, null (assumed native mm) for millimetres; get_Value is in this unit.</summary>
        private string? DocLengthUnit(ModelDoc2 model)
        {
            var u = model.GetUserPreferenceIntegerValue((int)swUserPreferenceIntegerValue_e.swUnitsLinear);
            switch (u)
            {
                case (int)swLengthUnit_e.swMM: return null;
                case (int)swLengthUnit_e.swCM: return "cm";
                case (int)swLengthUnit_e.swMETER: return "m";
                case (int)swLengthUnit_e.swINCHES: return "in";
                default:
                    _log.Warn("document length unit " + u + " not supported; assuming mm");
                    return null;
            }
        }

        private void ReadEquations(ModelDoc2 model, NativeDocument doc)
        {
            _dimEquations.Clear();
            var eq = model.GetEquationMgr();
            var count = eq.GetCount();
            var unit = DocLengthUnit(model);
            for (var i = 0; i < count; i++)
            {
                var text = eq.get_Equation(i);
                var m = EqLeft.Match(text ?? "");
                if (!m.Success)
                {
                    _log.Warn("equation " + i + " not understood: " + text);
                    continue;
                }
                var left = m.Groups[1].Value;
                var rhs = m.Groups[2].Value.Trim();
                var expr = Plain.IsMatch(rhs) ? null : rhs;
                if (eq.get_GlobalVariable(i))
                {
                    doc.Globals.Add(new NativeGlobal
                    {
                        Name = left,
                        Value = NativeQuantity.Of(Num(eq.get_Value(i), 9), unit),
                        Expression = expr,
                    });
                }
                else
                {
                    _dimEquations[left] = rhs;
                }
            }
            _log.Debug("equations: " + doc.Globals.Count + " globals, " + _dimEquations.Count + " dimension equations");
        }

        private void ReadDimensions(ModelDoc2 model, NativeDocument doc)
        {
            WalkDimensions(model, doc.Dimensions, null, null);
        }

        /// <summary>Appends every distinct dimension of <paramref name="model"/>; component dimensions carry the instance and the component file in the locator.</summary>
        internal void WalkDimensions(ModelDoc2 model, List<NativeDimension> into, string? component, string? file)
        {
            var seen = new HashSet<string>(StringComparer.Ordinal);
            for (var f = model.FirstFeature() as Feature; f != null; f = f.GetNextFeature() as Feature)
            {
                for (var dd = f.GetFirstDisplayDimension() as DisplayDimension; dd != null; dd = f.GetNextDisplayDimension(dd) as DisplayDimension)
                {
                    var dim = dd.GetDimension2(0) as Dimension;
                    if (dim == null || !seen.Add(dim.FullName))
                    {
                        continue;
                    }
                    var d = ReadDimension(dim, file);
                    d.Component = component;
                    into.Add(d);
                }
            }
        }

        private NativeDimension ReadDimension(Dimension dim, string? file = null)
        {
            var name = dim.Name + "@" + dim.FullName.Split('@')[1] + (file == null ? "" : "@" + file);
            var kind = DimensionUnits.KindOf(((IDimension)dim).GetType());
            var angular = kind == DimensionUnitKind.Angle;
            var raw = dim.SystemValue;
            var q = DimensionUnits.ToQuantity(raw, kind);
            _log.Debug("dimension " + name + " DrivenState=" + dim.DrivenState + " ReadOnly=" + dim.ReadOnly + " IsReference=" + dim.IsReference());
            if (dim.IsReference())
            {
                q.Reason = "driven dimension: value is a measurement, not an input";
            }
            _dimEquations.TryGetValue(name, out var expr);
            _log.Debug("dimension " + name + " = " + q.Text + (kind == DimensionUnitKind.Count ? "" : angular ? "rad" : "mm"));
            return new NativeDimension { Locator = name, Value = q, Expression = expr, Tolerance = kind != DimensionUnitKind.Length ? null : ReadTolerance(dim, raw) };
        }

        private Protocol.DimTolerance? ReadTolerance(Dimension dim, double value)
        {
            var tol = dim.Tolerance;
            if (tol == null)
            {
                return null;
            }
            switch (tol.Type)
            {
                case (int)swTolType_e.swTolSYMMETRIC:
                    return new Protocol.DimTolerance { Form = "symmetric", T = Mm(tol.GetMaxValue()) };
                case (int)swTolType_e.swTolBILAT:
                    return new Protocol.DimTolerance { Form = "bilateral", Upper = Mm(tol.GetMaxValue()), Lower = Mm(tol.GetMinValue()) };
                case (int)swTolType_e.swTolLIMIT:
                    return new Protocol.DimTolerance { Form = "limits", Min = Mm(value + tol.GetMinValue()), Max = Mm(value + tol.GetMaxValue()) };
                case (int)swTolType_e.swTolFIT:
                    var hole = tol.GetHoleFitValue();
                    return new Protocol.DimTolerance { Form = "fit", Class = string.IsNullOrEmpty(hole) ? tol.GetShaftFitValue() : hole };
                default:
                    return null;
            }
        }

        private void ReadProperties(ModelDoc2 model, NativeDocument doc)
        {
            ConfigPropertyReader.Apply(ConfigPropertyReader.Collect(new SwComPropertySource(model, _log), _log), doc);
        }

        private void ReadFrames(ModelDoc2 model, NativeDocument doc)
        {
            for (var f = model.FirstFeature() as Feature; f != null; f = f.GetNextFeature() as Feature)
            {
                if (f.GetTypeName2() != "CoordSys")
                {
                    continue;
                }
                var t = model.Extension.GetCoordinateSystemTransformByName(f.Name);
                if (t == null)
                {
                    _log.Warn("coordinate system " + f.Name + " has no transform");
                    continue;
                }
                doc.Frames.Add(new NativeFrame { Name = f.Name, Transform = Rows((double[])t.ArrayData) });
            }
        }

        private void ReadInstances(AssemblyDoc asm, NativeDocument doc)
        {
            foreach (var o in (object[]?)asm.GetComponents(true) ?? new object[0])
            {
                var c = (Component2)o;
                var t = c.Transform2;
                if (t == null)
                {
                    _log.Warn("component " + c.Name2 + " has no transform");
                    continue;
                }
                var reference = c.ComponentReference;
                doc.Instances.Add(new NativeInstance
                {
                    Component = c.Name2,
                    Document = c.GetPathName(),
                    Reference = string.IsNullOrEmpty(reference) ? null : reference,
                    Transform = Rows((double[])t.ArrayData),
                });
            }
        }

        /// <summary>ArrayData (rotation 0-8 row-major, translation 9-11 in metres) to twelve row-major 3x4 quantities.</summary>
        public static List<NativeQuantity> Rows(double[] a)
        {
            var rows = new List<NativeQuantity>(12);
            for (var r = 0; r < 3; r++)
            {
                for (var c = 0; c < 3; c++)
                {
                    rows.Add(NativeQuantity.Of(Num(a[r * 3 + c], 12), ""));
                }
                rows.Add(NativeQuantity.Of(Mm(a[9 + r]), "mm"));
            }
            return rows;
        }
    }
}

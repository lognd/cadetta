using System;
using System.Linq;
using System.Globalization;
using Cadetta.SolidWorks.Com.Guard;
using Cadetta.SolidWorks.Guard;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Push;
using SolidWorks.Interop.sldworks;
using SolidWorks.Interop.swconst;

namespace Cadetta.SolidWorks.Com.Push
{
    // frob:ticket 01M4SV2TR8ENTA87J1DDKCF816
    /// <summary>
    /// IPushDocument over a SOLIDWORKS model (solidworks.md 4). Native units are
    /// mm and degrees; the COM layer converts to metres and radians.
    /// </summary>
    public sealed class SwComPushDocument : IPushDocument
    {
        private const double Mm = 1000.0;
        private const double Deg = 180.0 / Math.PI;

        private readonly IModelDoc2 _model;
        private readonly Log _log;

        /// <summary>Creates an adapter over <paramref name="model"/> saved at <paramref name="path"/>.</summary>
        public SwComPushDocument(IModelDoc2 model, string path, Log log)
        {
            _model = model;
            _log = log;
            Path = path;
        }

        /// <summary>Path of the document file.</summary>
        public string Path { get; }

        /// <summary>True when the file is read-only on disk or locked by another session.</summary>
        public bool ReadOnly => _model.IsOpenedReadOnly();

        /// <summary>True when the model holds edits that were not made by this adapter.</summary>
        public bool UserDirty => !AdapterModified && _model.GetSaveFlag();

        /// <summary>True once this adapter has written to the model.</summary>
        public bool AdapterModified { get; set; }

        // ---- globals and links ---------------------------------------------------------

        private IEquationMgr Eq => _model.GetEquationMgr();

        /// <summary>Finds the equation whose left-hand side is <paramref name="lhs"/>; -1 when absent.</summary>
        private int FindEquation(string lhs, out string rhs)
        {
            var eq = Eq;
            var count = eq.GetCount();
            for (var i = 0; i < count; i++)
            {
                if (SplitEquation(eq.get_Equation(i), out var l, out var r) && string.Equals(l, lhs, StringComparison.Ordinal))
                {
                    rhs = r;
                    return i;
                }
            }
            rhs = "";
            return -1;
        }

        private static bool SplitEquation(string? text, out string lhs, out string rhs)
        {
            lhs = "";
            rhs = "";
            if (string.IsNullOrEmpty(text)) return false;
            var s = text!.Trim();
            if (s.Length < 3 || s[0] != '"') return false;
            var close = s.IndexOf('"', 1);
            if (close < 0) return false;
            var eqSign = s.IndexOf('=', close);
            if (eqSign < 0) return false;
            lhs = s.Substring(1, close - 1);
            rhs = s.Substring(eqSign + 1).Trim();
            return true;
        }

        private static string Num(double v) => v.ToString("R", CultureInfo.InvariantCulture);

        private string Quantity(double nativeValue, bool angle) => Num(nativeValue) + (angle ? "deg" : "mm");

        /// <summary>Reads a global's value; null when no global of that name exists.</summary>
        public double? GetGlobal(string name)
        {
            var i = FindEquation(name, out _);
            if (i < 0 || !Eq.get_GlobalVariable(i)) return null;
            return Eq.get_Value(i);
        }

        /// <summary>The expression defining a global that a push would replace (R006), or null for a plain value.</summary>
        public string? GetGlobalExpression(string name)
        {
            var i = FindEquation(name, out var rhs);
            if (i < 0 || !Eq.get_GlobalVariable(i)) return null;
            return PushGuard.ForeignExpression(rhs);
        }

        /// <summary>Writes the equation lhs = rhs verbatim, or removes the equation on lhs when rhs is null.</summary>
        public bool RestoreEquation(string lhs, string? rhs)
        {
            var i = FindEquation(lhs, out _);
            if (rhs == null)
            {
                if (i >= 0) Eq.Delete(i);
                return true;
            }
            var text = "\"" + lhs + "\" = " + rhs;
            if (i >= 0)
            {
                Eq.set_Equation(i, text);
                return true;
            }
            return Eq.Add2(-1, text, true) >= 0;
        }

        /// <summary>Creates or edits a global; an angle is not guessed, so globals take the length unit unless they already hold degrees.</summary>
        public bool SetGlobal(string name, double value)
        {
            var i = FindEquation(name, out var rhs);
            var angle = i >= 0 && rhs.EndsWith("deg", StringComparison.OrdinalIgnoreCase);
            var text = "\"" + name + "\" = " + Quantity(value, angle);
            if (i < 0)
            {
                var added = Eq.Add2(-1, text, true);
                _log.Debug("global add " + text + " -> " + added);
                return added >= 0;
            }
            Eq.set_Equation(i, text);
            _log.Debug("global set " + text);
            return true;
        }

        /// <summary>Deletes a global's equation; true when it existed and is gone.</summary>
        public bool DeleteGlobal(string name)
        {
            var i = FindEquation(name, out _);
            if (i < 0) return false;
            Eq.Delete(i);
            _log.Debug("global delete " + name);
            return true;
        }

        // ---- dimensions ---------------------------------------------------------------

        private IDimension? Dim(string name) => _model.Parameter(name) as IDimension;

        private static bool IsAngle(IDimension d)
        {
            var m = typeof(IDimension).GetMethod("GetType", Type.EmptyTypes);
            var kind = m == null ? 0 : (int)m.Invoke(d, null)!;
            return kind == (int)swDimensionType_e.swAngularDimension;
        }

        private static double ToNative(double system, bool angle) => angle ? system * Deg : system * Mm;

        private static double ToSystem(double native, bool angle) => angle ? native / Deg : native / Mm;

        /// <summary>Describes a dimension, or null when missing.</summary>
        public DimensionInfo? GetDimension(string name)
        {
            var d = Dim(name);
            if (d == null) return null;
            var angle = IsAngle(d);
            var info = new DimensionInfo
            {
                Value = ToNative(d.SystemValue, angle),
                Tolerance = ReadTolerance(d, angle),
            };
            var i = FindEquation(name, out var rhs);
            SwComGuardFacts.Fill(info, d, name, i >= 0 ? rhs : null, _log);
            return info;
        }

        /// <summary>Sets a dimension's value; false when SOLIDWORKS refuses it.</summary>
        public bool SetDimensionValue(string name, double value)
        {
            var d = Dim(name);
            if (d == null) return false;
            var status = d.SetSystemValue3(ToSystem(value, IsAngle(d)), (int)swSetValueInConfiguration_e.swSetValue_InAllConfigurations, null);
            _log.Debug("dimension " + name + " set " + Num(value) + " status " + status);
            return status == (int)swSetValueReturnStatus_e.swSetValue_Successful;
        }

        /// <summary>Links a dimension to a global through an equation, or removes the link when <paramref name="global"/> is null.</summary>
        public bool SetDimensionLink(string name, string? global)
        {
            var i = FindEquation(name, out _);
            if (global == null)
            {
                if (i < 0) return true;
                Eq.Delete(i);
                return true;
            }
            var text = "\"" + name + "\" = \"" + global + "\"";
            if (i >= 0)
            {
                Eq.set_Equation(i, text);
                return true;
            }
            var added = Eq.Add2(-1, text, true);
            _log.Debug("dimension link " + text + " -> " + added);
            return added >= 0;
        }

        private static ToleranceSpec ReadTolerance(IDimension d, bool angle)
        {
            var t = d.Tolerance;
            if (t == null) return ToleranceSpec.None;
            decimal Conv(double v) => Math.Round((decimal)ToNative(v, angle), 9);
            switch (t.Type)
            {
                case (int)swTolType_e.swTolSYMMETRIC:
                    return new ToleranceSpec(ToleranceKind.Symmetric, -Conv(t.GetMaxValue()), Conv(t.GetMaxValue()));
                case (int)swTolType_e.swTolBILAT:
                    return new ToleranceSpec(ToleranceKind.Bilateral, Conv(t.GetMinValue()), Conv(t.GetMaxValue()));
                case (int)swTolType_e.swTolLIMIT:
                    return new ToleranceSpec(ToleranceKind.Limit, Conv(t.GetMinValue()), Conv(t.GetMaxValue()));
                case (int)swTolType_e.swTolFIT:
                    string hole, shaft;
                    hole = t.GetHoleFitValue() ?? "";
                    shaft = t.GetShaftFitValue() ?? "";
                    return new ToleranceSpec(ToleranceKind.Fit, hole: hole, shaft: shaft);
                default:
                    return ToleranceSpec.None;
            }
        }

        /// <summary>Applies a tolerance to a dimension; false when SOLIDWORKS refuses a setting.</summary>
        public bool SetDimensionTolerance(string name, ToleranceSpec spec)
        {
            var d = Dim(name);
            if (d == null) return false;
            var angle = IsAngle(d);
            var t = d.Tolerance;
            double S(decimal v) => ToSystem((double)v, angle);
            bool ok;
            switch (spec.Kind)
            {
                case ToleranceKind.Symmetric:
                    t.Type = (int)swTolType_e.swTolSYMMETRIC;
                    ok = t.SetValues(S(spec.Upper), S(spec.Upper));
                    break;
                case ToleranceKind.Bilateral:
                    t.Type = (int)swTolType_e.swTolBILAT;
                    ok = t.SetValues(S(spec.Lower), S(spec.Upper));
                    break;
                case ToleranceKind.Limit:
                    t.Type = (int)swTolType_e.swTolLIMIT;
                    ok = t.SetValues(S(spec.Lower), S(spec.Upper));
                    break;
                case ToleranceKind.Fit:
                    t.Type = (int)swTolType_e.swTolFIT;
                    t.FitType = (int)swFitType_e.swFitUSER;
                    ok = t.SetFitValues(spec.Hole, spec.Shaft);
                    break;
                default:
                    t.Type = (int)swTolType_e.swTolNONE;
                    ok = true;
                    break;
            }
            _log.Debug("dimension " + name + " tolerance " + spec.Kind + " ok=" + ok);
            return ok;
        }

        // ---- properties ---------------------------------------------------------------

        private ICustomPropertyManager Props => _model.Extension.CustomPropertyManager[""];

        /// <summary>Reads a file-level custom property; null when absent.</summary>
        public string? GetProperty(string name)
        {
            var mgr = Props;
            // Get5 reports a value for absent names, so existence comes from the name list
            var names = mgr.GetNames() as object[];
            if (names == null || !names.Any(n => string.Equals(n as string, name, StringComparison.Ordinal))) return null;
            string val, resolved;
            bool was;
            mgr.Get5(name, false, out val, out resolved, out was);
            return val;
        }

        /// <summary>Adds or replaces a file-level custom property.</summary>
        public bool SetProperty(string name, string value)
        {
            var r = Props.Add3(name, (int)swCustomInfoType_e.swCustomInfoText, value,
                (int)swCustomPropertyAddOption_e.swCustomPropertyReplaceValue);
            _log.Debug("property " + name + " add3 " + r);
            return r >= 0;
        }

        /// <summary>Deletes a file-level custom property.</summary>
        public bool DeleteProperty(string name)
        {
            return Props.Delete2(name) != (int)swCustomInfoDeleteResult_e.swCustomInfoDeleteResult_LinkedProp;
        }

        // ---- frames -------------------------------------------------------------------

        /// <summary>Reads a coordinate system as a row-major 3x4 (SOLIDWORKS ArrayData layout, translation in mm).</summary>
        public double[]? GetFrame(string name)
        {
            var t = _model.Extension.GetCoordinateSystemTransformByName(name);
            if (t == null) return null;
            var a = (double[])t.ArrayData;
            var m = new double[12];
            for (var r = 0; r < 3; r++)
            {
                for (var c = 0; c < 3; c++) m[r * 4 + c] = a[r * 3 + c];
                m[r * 4 + 3] = a[9 + r] * Mm;
            }
            return m;
        }

        /// <summary>Creates or replaces a coordinate system from the 3x4 transform (numeric values; adapters.md 3.4).</summary>
        public bool SetFrame(string name, double[] m, bool create)
        {
            var exists = _model.Extension.GetCoordinateSystemTransformByName(name) != null;
            if (!exists && !create) return false;
            if (exists && !DeleteFrame(name)) return false;
            // ArrayData is the transpose of the column-vector rotation Rz*Ry*Rx.
            double R(int row, int col) => m[col * 4 + row];
            var ry = Math.Asin(Math.Max(-1.0, Math.Min(1.0, -R(2, 0))));
            double rx, rz;
            if (Math.Abs(Math.Cos(ry)) < 1e-9)
            {
                rx = 0;
                rz = Math.Atan2(-R(0, 1), R(1, 1));
            }
            else
            {
                rx = Math.Atan2(R(2, 1), R(2, 2));
                rz = Math.Atan2(R(1, 0), R(0, 0));
            }
            var f = _model.FeatureManager.CreateCoordinateSystemUsingNumericalValues(
                true, m[3] / Mm, m[7] / Mm, m[11] / Mm, true, rx, ry, rz);
            if (f == null)
            {
                _log.Warn("coordinate system " + name + " could not be created");
                return false;
            }
            f.Name = name;
            _log.Debug("coordinate system " + name + " written");
            return true;
        }

        /// <summary>Deletes a coordinate system feature by name.</summary>
        public bool DeleteFrame(string name)
        {
            _model.ClearSelection2(true);
            var picked = _model.Extension.SelectByID2(name, "COORDSYS", 0, 0, 0, false, 0, null, 0);
            if (!picked) return false;
            var ok = _model.Extension.DeleteSelection2(0);
            _log.Debug("coordinate system " + name + " delete " + ok);
            return ok;
        }

        // ---- rebuild ------------------------------------------------------------------

        /// <summary>Forces a full rebuild; false when it reports errors.</summary>
        public bool Rebuild()
        {
            var ok = _model.ForceRebuild3(false);
            AdapterModified = true;
            _log.Debug("rebuild ok: " + ok);
            return ok;
        }
    }
}

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Numerics;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.Read
{
    /// <summary>
    /// Turns native SOLIDWORKS documents into observed `cadetta.ir/1`: one
    /// synthetic component of params with imported provenance, one binding per
    /// member, and frames[] / instances[] (adapters.md 3, 3.4).
    /// frob:ticket 01M4SV2TKZD26CK9FFA1EJXSV4
    /// </summary>
    public sealed class ObservedIrBuilder
    {
        private const string ComponentName = "Observed";
        private const int ExactDigits = 15;
        private static readonly Regex Decimal = new Regex(@"^([+-]?)(\d*)(?:\.(\d*))?(?:[eE]([+-]?\d+))?$", RegexOptions.Compiled);

        private readonly string _package;
        private readonly Log _log;
        private readonly HashSet<string> _names = new HashSet<string>(StringComparer.Ordinal);
        private readonly JsonArray _members = new JsonArray();
        private readonly JsonArray _bindings = new JsonArray();
        private readonly JsonArray _frames = new JsonArray();
        private readonly JsonArray _instances = new JsonArray();
        private readonly List<Protocol.ObservedTolerance> _tolerances = new List<Protocol.ObservedTolerance>();

        /// <summary>Starts an observed project named after <paramref name="firstFile"/>.</summary>
        public ObservedIrBuilder(string firstFile, Log log)
        {
            _package = PackageName(firstFile);
            _log = log;
        }

        /// <summary>Lowercase identifier-safe stem of a file path; `observed` when empty.</summary>
        public static string PackageName(string file)
        {
            var stem = System.IO.Path.GetFileNameWithoutExtension(file.Replace('\\', '/'));
            var name = Regex.Replace(stem.ToLowerInvariant(), "[^a-z0-9_]", "_").Trim('_');
            if (name.Length == 0)
            {
                return "observed";
            }
            return char.IsDigit(name[0]) ? "p" + name : name;
        }

        /// <summary>Path-safe identifier: non-alphanumerics become `_`, a leading digit gets a `_` prefix.</summary>
        public static string Ident(string native)
        {
            var sb = new StringBuilder();
            foreach (var c in native)
            {
                sb.Append(c < 128 && (char.IsLetterOrDigit(c) || c == '_') ? c : '_');
            }
            if (sb.Length == 0 || char.IsDigit(sb[0]))
            {
                sb.Insert(0, '_');
            }
            return sb.ToString();
        }

        /// <summary>Adds every global, dimension, property, frame and instance of <paramref name="doc"/>.</summary>
        public void Add(NativeDocument doc)
        {
            var file = System.IO.Path.GetFileName(doc.Path.Replace('\\', '/'));
            _log.Info("observe " + doc.Path + ": " + doc.Globals.Count + " globals, " + doc.Dimensions.Count
                + " dimensions, " + doc.Properties.Count + " properties, " + doc.Frames.Count + " frames, "
                + doc.Instances.Count + " instances");
            foreach (var g in doc.Globals)
            {
                AddMember(file, g.Name, "global", null, g.Value, g.Expression);
            }
            foreach (var d in doc.Dimensions)
            {
                AddMember(file, d.Locator, "dimension", null, d.Value, d.Expression);
                _tolerances.Add(new Protocol.ObservedTolerance { Locator = d.Locator, Tolerance = d.Tolerance });
            }
            foreach (var p in doc.Properties)
            {
                AddMember(file, p.Name, "property", p.Config, PropertyQuantity(p), p.Expression);
            }
            foreach (var f in doc.Frames)
            {
                _frames.Add(new JsonObject
                {
                    ["document"] = file,
                    ["name"] = f.Name,
                    ["transform"] = Transform(f.Transform, f.Name),
                });
            }
            foreach (var i in doc.Instances)
            {
                var o = new JsonObject { ["component"] = i.Component, ["document"] = i.Document };
                if (i.Reference != null)
                {
                    o["reference"] = i.Reference;
                }
                o["transform"] = Transform(i.Transform, i.Component);
                _instances.Add(o);
            }
        }

        /// <summary>Finishes: the observed project plus the dimension tolerances read.</summary>
        public Protocol.ReadResult Build()
        {
            var comp = new JsonObject
            {
                ["attrs"] = new JsonObject(),
                ["class"] = "model",
                ["fits"] = new JsonArray(),
                ["implements"] = new JsonArray(),
                ["members"] = _members,
                ["parts"] = new JsonArray(),
                ["path"] = _package + "::" + ComponentName,
            };
            var root = new JsonObject
            {
                ["schema"] = "cadetta.ir/1",
                ["flavour"] = "observed",
                ["package"] = new JsonObject { ["name"] = _package, ["version"] = "0.0.0" },
                ["modules"] = new JsonArray(),
                ["dependencies"] = new JsonArray(),
                ["components"] = new JsonArray(comp),
                ["citations"] = new JsonArray(),
                ["checks"] = new JsonArray(),
                ["solvers"] = new JsonArray(),
                ["bindings"] = _bindings,
            };
            if (_frames.Count > 0)
            {
                root["frames"] = _frames;
            }
            if (_instances.Count > 0)
            {
                root["instances"] = _instances;
            }
            using var doc = JsonDocument.Parse(root.ToJsonString());
            _log.Info("observed built: " + _members.Count + " members");
            return new Protocol.ReadResult { Observed = doc.RootElement.Clone(), Tolerances = _tolerances };
        }

        private NativeQuantity PropertyQuantity(NativeProperty p)
        {
            if (Decimal.IsMatch(p.Value.Trim()) && p.Value.Trim().Length > 0 && p.Value.Trim() != ".")
            {
                return NativeQuantity.Of(p.Value.Trim());
            }
            _log.Warn("property " + p.Name + " is not numeric: " + p.Value);
            return NativeQuantity.Of("0", "", "property value is text, not a number: " + p.Value);
        }

        private void AddMember(string file, string native, string kind, string? config, NativeQuantity q, string? expr)
        {
            var id = Ident(native);
            var name = id;
            for (var n = 1; !_names.Add(name);)
            {
                name = id + "_" + (++n);
            }
            var path = _package + "::" + ComponentName + "." + name;
            var nativeUnit = string.IsNullOrEmpty(q.Unit) ? AdapterInfo.NativeUnits : q.Unit;
            var value = Value(q, path);
            if (q.Reason != null)
            {
                _log.Warn("observed " + native + " not faithful: " + q.Reason);
            }
            var member = new JsonObject
            {
                ["analysis"] = false,
                ["attrs"] = new JsonObject(),
                ["deps"] = new JsonArray(),
                ["kind"] = "param",
                ["path"] = path,
                ["priv"] = false,
                ["provenance"] = new JsonObject
                {
                    ["imported"] = new JsonObject
                    {
                        ["adapter"] = AdapterInfo.Name,
                        ["file"] = file,
                        ["native_name"] = native,
                    },
                },
                ["was"] = new JsonArray(),
                ["value"] = value,
            };
            if (expr != null)
            {
                member["expr"] = expr;
            }
            _members.Add(member);
            var loc = new JsonObject
            {
                ["kind"] = kind,
                ["locator"] = native,
                ["mode"] = "value",
                ["native_unit"] = nativeUnit,
            };
            if (config != null)
            {
                loc["config"] = config;
            }
            _bindings.Add(new JsonObject
            {
                ["adapter"] = AdapterInfo.Name,
                ["path"] = path,
                ["locators"] = new JsonArray(loc),
            });
            _log.Debug("observed " + kind + " " + native + " as " + path);
        }

        private JsonArray Transform(List<NativeQuantity> t, string owner)
        {
            if (t.Count != 12)
            {
                throw new ArgumentException("transform of " + owner + " needs 12 entries, got " + t.Count);
            }
            var rows = new JsonArray();
            for (var r = 0; r < 3; r++)
            {
                var row = new JsonArray();
                for (var c = 0; c < 4; c++)
                {
                    row.Add(Value(t[r * 4 + c], _package + "::" + ComponentName));
                }
                rows.Add(row);
            }
            return rows;
        }

        private struct UnitInfo
        {
            public BigInteger Num;
            public BigInteger Den;
            public int Pi;
            public int Length;
            public int Angle;
        }

        private static bool TryUnit(string unit, out UnitInfo u)
        {
            u = new UnitInfo { Num = 1, Den = 1 };
            switch (unit)
            {
                case "mm": u.Den = 1000; u.Length = 1; return true;
                case "cm": u.Den = 100; u.Length = 1; return true;
                case "m": u.Length = 1; return true;
                case "in": u.Num = 127; u.Den = 5000; u.Length = 1; return true;
                case "deg": u.Den = 180; u.Pi = 1; u.Angle = 1; return true;
                case "rad": u.Angle = 1; return true;
                case "": return true;
                default: return false;
            }
        }

        /// <summary>Parses a decimal literal into an exact fraction and its significant digit count.</summary>
        public static bool TryRational(string text, out BigInteger num, out BigInteger den, out int digits)
        {
            num = 0;
            den = 1;
            digits = 0;
            var m = Decimal.Match(text.Trim());
            if (!m.Success || (m.Groups[2].Value.Length == 0 && m.Groups[3].Value.Length == 0))
            {
                return false;
            }
            var frac = m.Groups[3].Value;
            var all = (m.Groups[2].Value + frac).TrimStart('0');
            digits = all.Length;
            num = all.Length == 0 ? BigInteger.Zero : BigInteger.Parse(all, CultureInfo.InvariantCulture);
            var exp = (m.Groups[4].Success ? int.Parse(m.Groups[4].Value, CultureInfo.InvariantCulture) : 0) - frac.Length;
            if (Math.Abs(exp) > 400)
            {
                return false;
            }
            if (exp >= 0)
            {
                num *= BigInteger.Pow(10, exp);
            }
            else
            {
                den = BigInteger.Pow(10, -exp);
            }
            if (m.Groups[1].Value == "-")
            {
                num = -num;
            }
            return true;
        }

        private JsonObject Value(NativeQuantity q, string root)
        {
            var assumed = q.Unit == null;
            var unit = q.Unit ?? AdapterInfo.NativeUnits;
            var reason = q.Reason;
            var knownUnit = TryUnit(unit, out var u);
            if (!knownUnit)
            {
                u = new UnitInfo { Num = 1, Den = 1 };
                reason ??= "unknown unit '" + unit + "'";
            }
            var parsed = TryRational(q.Text, out var num, out var den, out var digits);
            if (!parsed)
            {
                num = 0;
                den = 1;
                reason ??= "value '" + q.Text + "' is not a decimal number";
            }
            else if (digits > ExactDigits)
            {
                reason ??= "binary64 value " + q.Text + " is not a short decimal";
            }
            var exact = reason == null;
            var rn = num * u.Num;
            var rd = den * u.Den;
            var g = BigInteger.GreatestCommonDivisor(rn, rd);
            if (!g.IsOne && !g.IsZero)
            {
                rn /= g;
                rd /= g;
            }
            JsonObject nominal;
            if (parsed && digits > ExactDigits)
            {
                var f = (double.Parse(q.Text.Trim(), NumberStyles.Float, CultureInfo.InvariantCulture)
                    * (double)u.Num / (double)u.Den) * (u.Pi == 1 ? Math.PI : 1.0);
                nominal = new JsonObject { ["f64"] = f, ["root"] = root };
            }
            else
            {
                nominal = new JsonObject { ["pi"] = u.Pi, ["r"] = rn.ToString(CultureInfo.InvariantCulture) + "/" + rd.ToString(CultureInfo.InvariantCulture) };
            }
            var value = new JsonObject
            {
                ["angle"] = 0,
                ["authored"] = q.Text.Trim() + unit,
                ["conservative"] = false,
                ["dim"] = new JsonObject
                {
                    ["A"] = 0,
                    ["K"] = 0,
                    ["angle"] = u.Angle,
                    ["cd"] = 0,
                    ["currency"] = new JsonObject(),
                    ["kg"] = 0,
                    ["m"] = u.Length,
                    ["mol"] = 0,
                    ["s"] = 0,
                },
                ["exact"] = exact,
                ["nominal"] = nominal,
            };
            if (reason != null)
            {
                value["reason"] = reason;
            }
            if (assumed)
            {
                value["unit_assumed"] = unit;
            }
            return value;
        }
    }
}

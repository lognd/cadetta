using System;
using System.Globalization;
using System.Numerics;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace Cadetta.SolidWorks.Push
{
    // frob:ticket 01M4SV2TR8ENTA87J1DDKCF816
    /// <summary>A pushed number as exact decimal text at the declared precision, or why it cannot be written.</summary>
    public sealed class PushNumber
    {
        /// <summary>Diagnostic code for a value not representable at the declared precision (adapters.md 4 rule 3).</summary>
        public const string Q004 = "Q004";

        private PushNumber(string? text, string? code, string? error)
        {
            Text = text;
            Code = code;
            Error = error;
        }

        /// <summary>Decimal text in the native unit, trailing zeros trimmed; null on failure.</summary>
        public string? Text { get; }

        /// <summary>The value as a double (for the native API); only meaningful when <see cref="Text"/> is set.</summary>
        public double Value => Text == null ? 0 : double.Parse(Text, CultureInfo.InvariantCulture);

        /// <summary>The diagnostic code of the failure (Q004 or R010), or null on success.</summary>
        public string? Code { get; }

        /// <summary>The failure message, or null on success.</summary>
        public string? Error { get; }

        /// <summary>True when the value can be written.</summary>
        public bool Ok => Text != null;

        private static readonly Regex NumberRe = new Regex(@"^\s*(-?)(\d+)(?:\.(\d+))?(?:[eE]([+-]?\d+))?\s*([A-Za-z]*)\s*$", RegexOptions.CultureInvariant);
        private static readonly Regex RatioRe = new Regex(@"^\s*(-?\d+)\s*/\s*(\d+)\s*([A-Za-z]*)\s*$", RegexOptions.CultureInvariant);

        /// <summary>Parses a wire value (number, "12 mm", "1/4in" or an exact {"r","pi"} object) at <paramref name="precision"/> decimals.</summary>
        public static PushNumber Parse(JsonElement? value, int precision)
        {
            if (value == null)
            {
                return Fail("R010", "no value given");
            }
            var v = value.Value;
            switch (v.ValueKind)
            {
                case JsonValueKind.Number:
                    return FromText(v.GetRawText(), precision);
                case JsonValueKind.String:
                    return FromText(v.GetString() ?? "", precision);
                case JsonValueKind.Object:
                    var nominal = v.TryGetProperty("nominal", out var n) ? n : v;
                    return FromObject(nominal, precision);
                default:
                    return Fail("R010", "value " + v.GetRawText() + " is not a number");
            }
        }

        /// <summary>Parses decimal or ratio text with an optional length unit.</summary>
        public static PushNumber FromText(string text, int precision)
        {
            var m = NumberRe.Match(text);
            if (m.Success)
            {
                var frac = m.Groups[3].Value;
                var num = BigInteger.Parse(m.Groups[2].Value + frac, CultureInfo.InvariantCulture);
                var den = BigInteger.Pow(10, frac.Length);
                if (m.Groups[4].Success)
                {
                    var e = int.Parse(m.Groups[4].Value, CultureInfo.InvariantCulture);
                    if (e >= 0) num *= BigInteger.Pow(10, e); else den *= BigInteger.Pow(10, -e);
                }
                if (m.Groups[1].Value == "-") num = -num;
                return Scaled(num, den, m.Groups[5].Value, precision, text);
            }
            var r = RatioRe.Match(text);
            if (r.Success && BigInteger.Parse(r.Groups[2].Value) != 0)
            {
                return Scaled(BigInteger.Parse(r.Groups[1].Value), BigInteger.Parse(r.Groups[2].Value), r.Groups[3].Value, precision, text);
            }
            return Fail("R010", "value '" + text + "' is not numeric");
        }

        private static PushNumber FromObject(JsonElement nominal, int precision)
        {
            if (nominal.ValueKind == JsonValueKind.Object && nominal.TryGetProperty("r", out var r) && r.ValueKind == JsonValueKind.String)
            {
                if (nominal.TryGetProperty("pi", out var pi) && pi.ValueKind == JsonValueKind.Number && pi.GetInt32() != 0)
                {
                    return Fail(Q004, Q004 + ": " + nominal.GetRawText() + " carries a power of pi and is not representable at " + precision + " decimals");
                }
                var parts = (r.GetString() ?? "").Split('/');
                if (parts.Length == 2 && BigInteger.TryParse(parts[0], out var a) && BigInteger.TryParse(parts[1], out var b) && b > 0)
                {
                    return Scaled(a, b, "", precision, nominal.GetRawText());
                }
                return Fail("R010", "ratio '" + r.GetString() + "' is malformed");
            }
            if (nominal.ValueKind == JsonValueKind.Object && nominal.TryGetProperty("f64", out var f) && f.ValueKind == JsonValueKind.Number)
            {
                return FromText(f.GetRawText(), precision);
            }
            if (nominal.ValueKind == JsonValueKind.Number || nominal.ValueKind == JsonValueKind.String)
            {
                return Parse(nominal, precision);
            }
            return Fail("R010", "value " + nominal.GetRawText() + " is not a number");
        }

        private static PushNumber Scaled(BigInteger num, BigInteger den, string unit, int precision, string source)
        {
            if (!UnitScale(unit, out var un, out var ud))
            {
                return Fail("R010", "unit '" + unit + "' is not supported for a push");
            }
            num *= un;
            den *= ud;
            return Format(num, den, precision, source);
        }

        private static bool UnitScale(string unit, out BigInteger num, out BigInteger den)
        {
            num = 1;
            den = 1;
            switch (unit)
            {
                case "":
                case "mm":
                case "deg":
                    return true;
                case "cm":
                    num = 10;
                    return true;
                case "m":
                    num = 1000;
                    return true;
                case "in":
                    num = 127;
                    den = 5;
                    return true;
                default:
                    return false;
            }
        }

        /// <summary>Formats num/den with at most <paramref name="precision"/> places; Q004 when it does not fit exactly.</summary>
        public static PushNumber Format(BigInteger num, BigInteger den, int precision, string source)
        {
            if (den.Sign < 0)
            {
                num = -num;
                den = -den;
            }
            var scale = BigInteger.Pow(10, precision);
            var scaled = num * scale;
            if (!(scaled % den).IsZero)
            {
                return Fail(Q004, Q004 + ": " + source + " is not representable at " + precision + " decimals");
            }
            var n = scaled / den;
            var neg = n.Sign < 0;
            n = BigInteger.Abs(n);
            var whole = BigInteger.DivRem(n, scale, out var frac);
            var sb = new StringBuilder();
            if (neg && !(whole.IsZero && frac.IsZero)) sb.Append('-');
            sb.Append(whole.ToString(CultureInfo.InvariantCulture));
            if (!frac.IsZero)
            {
                sb.Append('.').Append(frac.ToString(CultureInfo.InvariantCulture).PadLeft(precision, '0').TrimEnd('0'));
            }
            return new PushNumber(sb.ToString(), null, null);
        }

        /// <summary>Renders a decimal with trailing zeros trimmed and no negative zero.</summary>
        public static string Trim(decimal d)
        {
            var t = d.ToString("0.############################", CultureInfo.InvariantCulture);
            return t == "-0" ? "0" : t;
        }

        private static PushNumber Fail(string code, string message) => new PushNumber(null, code, message);
    }
}

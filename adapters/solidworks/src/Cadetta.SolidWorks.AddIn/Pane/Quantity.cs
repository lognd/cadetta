// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
using System;
using System.Collections.Generic;
using System.Globalization;

namespace Cadetta.SolidWorks.AddIn.Pane
{
    /// <summary>Dimension kind of a quantity, as far as the pane needs to tell paths apart.</summary>
    public enum QuantityKind
    {
        /// <summary>Not recognised.</summary>
        Unknown = 0,
        /// <summary>A length.</summary>
        Length = 1,
        /// <summary>A plane angle.</summary>
        Angle = 2,
        /// <summary>A bare number (a count).</summary>
        Count = 3,
    }

    /// <summary>A parsed emit text: kind and value in SI (metres, radians, or the bare number).</summary>
    public readonly struct Quantity
    {
        /// <summary>Creates a quantity.</summary>
        public Quantity(QuantityKind kind, double si)
        {
            Kind = kind;
            Si = si;
        }

        /// <summary>The kind.</summary>
        public QuantityKind Kind { get; }

        /// <summary>The value in SI.</summary>
        public double Si { get; }

        private static readonly Dictionary<string, (QuantityKind, double)> Units = new Dictionary<string, (QuantityKind, double)>
        {
            ["m"] = (QuantityKind.Length, 1.0),
            ["cm"] = (QuantityKind.Length, 0.01),
            ["mm"] = (QuantityKind.Length, 0.001),
            ["um"] = (QuantityKind.Length, 1e-6),
            ["in"] = (QuantityKind.Length, 0.0254),
            ["ft"] = (QuantityKind.Length, 0.3048),
            ["thou"] = (QuantityKind.Length, 0.0000254),
            ["rad"] = (QuantityKind.Angle, 1.0),
            ["deg"] = (QuantityKind.Angle, Math.PI / 180.0),
            ["°"] = (QuantityKind.Angle, Math.PI / 180.0),
        };

        /// <summary>Parses text such as "6mm", "6 mm", "90deg" or "4"; Unknown kind when it is none of those.</summary>
        public static Quantity Parse(string? text)
        {
            if (string.IsNullOrWhiteSpace(text))
            {
                return new Quantity(QuantityKind.Unknown, 0);
            }
            var s = text!.Trim();
            var i = 0;
            while (i < s.Length && (char.IsDigit(s[i]) || s[i] == '.' || s[i] == '-' || s[i] == '+'
                || ((s[i] == 'e' || s[i] == 'E') && i + 1 < s.Length && (char.IsDigit(s[i + 1]) || s[i + 1] == '-' || s[i + 1] == '+') && i > 0)))
            {
                i++;
            }
            if (i == 0 || !double.TryParse(s.Substring(0, i), NumberStyles.Float, CultureInfo.InvariantCulture, out var n))
            {
                return new Quantity(QuantityKind.Unknown, 0);
            }
            var unit = s.Substring(i).Trim();
            if (unit.Length == 0)
            {
                return new Quantity(QuantityKind.Count, n);
            }
            return Units.TryGetValue(unit, out var u)
                ? new Quantity(u.Item1, n * u.Item2)
                : new Quantity(QuantityKind.Unknown, 0);
        }
    }
}

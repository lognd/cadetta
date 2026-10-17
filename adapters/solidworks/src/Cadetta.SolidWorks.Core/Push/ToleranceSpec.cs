using System;
using Cadetta.SolidWorks.Protocol;

namespace Cadetta.SolidWorks.Push
{
    // frob:ticket 01M4SV2TR8ENTA87J1DDKCF816
    /// <summary>The SOLIDWORKS tolerance types a design tolerance maps to (solidworks.md 4).</summary>
    public enum ToleranceKind
    {
        /// <summary>No tolerance (swTolNONE).</summary>
        None,

        /// <summary>Plus and minus the same amount (swTolSYMMETRIC).</summary>
        Symmetric,

        /// <summary>Separate upper and lower deviations (swTolBILAT).</summary>
        Bilateral,

        /// <summary>Limits shown as the two extreme values (swTolLIMIT).</summary>
        Limit,

        /// <summary>An ISO 286 fit class (swTolFIT).</summary>
        Fit,
    }

    /// <summary>A dimension tolerance in SOLIDWORKS terms, in the native unit: deviations from the dimension value, or a fit class.</summary>
    public sealed class ToleranceSpec : IEquatable<ToleranceSpec>
    {
        /// <summary>Creates a spec; unused members stay zero or empty.</summary>
        public ToleranceSpec(ToleranceKind kind, decimal lower = 0, decimal upper = 0, string hole = "", string shaft = "")
        {
            Kind = kind;
            Lower = lower;
            Upper = upper;
            Hole = hole;
            Shaft = shaft;
        }

        /// <summary>The tolerance type.</summary>
        public ToleranceKind Kind { get; }

        /// <summary>Lower deviation, signed (symmetric: minus t).</summary>
        public decimal Lower { get; }

        /// <summary>Upper deviation, signed (symmetric: plus t).</summary>
        public decimal Upper { get; }

        /// <summary>Hole fit class such as H7, or empty.</summary>
        public string Hole { get; }

        /// <summary>Shaft fit class such as g6, or empty.</summary>
        public string Shaft { get; }

        /// <summary>The no-tolerance spec.</summary>
        public static ToleranceSpec None { get; } = new ToleranceSpec(ToleranceKind.None);

        /// <summary>Value equality at the spec's own decimals.</summary>
        public bool Equals(ToleranceSpec? other) =>
            other != null && Kind == other.Kind && Lower == other.Lower && Upper == other.Upper && Hole == other.Hole && Shaft == other.Shaft;

        /// <inheritdoc />
        public override bool Equals(object? obj) => Equals(obj as ToleranceSpec);

        /// <inheritdoc />
        public override int GetHashCode() => (Kind, Lower, Upper, Hole, Shaft).GetHashCode();

        /// <summary>Renders the spec back to the protocol form for the per-locator status read-back (adapters.md 3.6).</summary>
        public DimTolerance? ToProtocol(decimal value)
        {
            switch (Kind)
            {
                case ToleranceKind.Symmetric:
                    return new DimTolerance { Form = "symmetric", T = Dec(Upper) };
                case ToleranceKind.Bilateral:
                    return new DimTolerance { Form = "bilateral", Upper = Dec(Upper), Lower = Dec(Lower) };
                case ToleranceKind.Limit:
                    return new DimTolerance { Form = "limits", Min = Dec(value + Lower), Max = Dec(value + Upper) };
                case ToleranceKind.Fit:
                    return new DimTolerance { Form = "fit", Class = Hole.Length > 0 ? Hole : Shaft };
                default:
                    return null;
            }
        }

        private static string Dec(decimal d) => PushNumber.Trim(d);
    }
}

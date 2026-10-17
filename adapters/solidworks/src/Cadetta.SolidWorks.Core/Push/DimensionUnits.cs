using System;
using System.Globalization;
using Cadetta.SolidWorks.Read;

namespace Cadetta.SolidWorks.Push
{
    /// <summary>The unit family of a SOLIDWORKS dimension, which decides how its system value maps to the native value.</summary>
    public enum DimensionUnitKind
    {
        /// <summary>A length in metres (system) shown in millimetres (native).</summary>
        Length,

        /// <summary>An angle in radians (system) shown in degrees (native).</summary>
        Angle,

        /// <summary>A unitless scalar such as a pattern instance count; system and native agree.</summary>
        Count,
    }

    // frob:ticket 01M559F56CVTCX37155XMNTFDK
    /// <summary>Converts dimension values between SOLIDWORKS system units and the native unit by dimension kind.</summary>
    public static class DimensionUnits
    {
        private const double Mm = 1000.0;
        private const double Deg = 180.0 / Math.PI;

        /// <summary>The unit kind for an IDimension.GetType() swDimensionParamType_e value (1 is angular, 2 is integer such as a pattern count, anything else a length).</summary>
        public static DimensionUnitKind KindOf(int swDimensionType) => swDimensionType switch
        {
            1 => DimensionUnitKind.Angle,
            2 => DimensionUnitKind.Count,
            _ => DimensionUnitKind.Length,
        };

        /// <summary>The native value for a system value of <paramref name="kind"/>.</summary>
        public static double ToNative(double system, DimensionUnitKind kind) => kind switch
        {
            DimensionUnitKind.Angle => system * Deg,
            DimensionUnitKind.Count => system,
            _ => system * Mm,
        };

        /// <summary>The system value for a native value of <paramref name="kind"/>.</summary>
        public static double ToSystem(double native, DimensionUnitKind kind) => kind switch
        {
            DimensionUnitKind.Angle => native / Deg,
            DimensionUnitKind.Count => native,
            _ => native / Mm,
        };

        // frob:ticket 01M559YQJ77Y2MGVMXYGFQKERH
        /// <summary>A plain number literal rounded to <paramref name="places"/> decimals (no negative zero).</summary>
        public static string Literal(double v, int places)
        {
            var r = Math.Round(v, places);
            return (r == 0 ? 0.0 : r).ToString("R", CultureInfo.InvariantCulture);
        }

        // frob:ticket 01M559YQJ77Y2MGVMXYGFQKERH
        /// <summary>The read-side quantity for a system value: mm for lengths, rad for angles, a unitless number for counts.</summary>
        public static NativeQuantity ToQuantity(double system, DimensionUnitKind kind) => kind switch
        {
            DimensionUnitKind.Angle => NativeQuantity.Of(Literal(system, 12), "rad"),
            DimensionUnitKind.Count => NativeQuantity.Of(Literal(system, 9), ""),
            _ => NativeQuantity.Of(Literal(ToNative(system, kind), 9)),
        };
    }
}

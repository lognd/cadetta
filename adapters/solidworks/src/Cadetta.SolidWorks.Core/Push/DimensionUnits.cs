using System;

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

        /// <summary>The unit kind for a swDimensionType_e value (3 is angular, 13 is scalar, anything else a length).</summary>
        public static DimensionUnitKind KindOf(int swDimensionType) => swDimensionType switch
        {
            3 => DimensionUnitKind.Angle,
            13 => DimensionUnitKind.Count,
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
    }
}

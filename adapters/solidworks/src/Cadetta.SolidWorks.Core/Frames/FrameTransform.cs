using System;
using System.Collections.Generic;
using System.Globalization;

namespace Cadetta.SolidWorks.Frames
{
    // frob:ticket 01M4SWMDXKYAFXR9GEKVTE7XA4
    /// <summary>
    /// A rigid transform: a row-major 3x3 rotation whose columns are the frame
    /// axes, plus a translation. Units are the caller's (metres from the API,
    /// the native unit after <see cref="Scaled"/>); adapters.md 3.4.
    /// </summary>
    public sealed class FrameTransform
    {
        /// <summary>Row-major 3x3 rotation; column j is the frame's j-th axis in the parent frame.</summary>
        public double[] Rotation { get; }

        /// <summary>Translation x.</summary>
        public double X { get; }

        /// <summary>Translation y.</summary>
        public double Y { get; }

        /// <summary>Translation z.</summary>
        public double Z { get; }

        /// <summary>Creates a transform from a row-major rotation and a translation.</summary>
        public FrameTransform(double[] rotation, double x, double y, double z)
        {
            if (rotation.Length != 9)
            {
                throw new ArgumentException("rotation needs 9 entries", nameof(rotation));
            }
            Rotation = (double[])rotation.Clone();
            X = x;
            Y = y;
            Z = z;
        }

        /// <summary>The identity transform.</summary>
        public static FrameTransform Identity { get; } =
            new FrameTransform(new double[] { 1, 0, 0, 0, 1, 0, 0, 0, 1 }, 0, 0, 0);

        /// <summary>Reads SOLIDWORKS MathTransform.ArrayData: rotation rows 0-8 hold the axes, translation 9-11 (API-SURFACE.md).</summary>
        public static FrameTransform FromSwArray(IReadOnlyList<double> a)
        {
            if (a.Count < 12)
            {
                throw new ArgumentException("ArrayData needs at least 12 entries", nameof(a));
            }
            var r = new double[9];
            for (var row = 0; row < 3; row++)
            {
                for (var col = 0; col < 3; col++)
                {
                    // ArrayData row i is axis i, so it is the transpose of the column-axis form.
                    r[row * 3 + col] = a[col * 3 + row];
                }
            }
            return new FrameTransform(r, a[9], a[10], a[11]);
        }

        /// <summary>Writes the 16-entry MathTransform.ArrayData form (scale 1, zero padding).</summary>
        public double[] ToSwArray()
        {
            var a = new double[16];
            for (var row = 0; row < 3; row++)
            {
                for (var col = 0; col < 3; col++)
                {
                    a[col * 3 + row] = Rotation[row * 3 + col];
                }
            }
            a[9] = X;
            a[10] = Y;
            a[11] = Z;
            a[12] = 1;
            return a;
        }

        /// <summary>Reads the protocol's 3x4 form: three rows of four, rotation columns then translation.</summary>
        public static FrameTransform FromRows(IReadOnlyList<double> rows)
        {
            if (rows.Count != 12)
            {
                throw new ArgumentException("a 3x4 transform needs 12 entries", nameof(rows));
            }
            var r = new double[9];
            for (var row = 0; row < 3; row++)
            {
                for (var col = 0; col < 3; col++)
                {
                    r[row * 3 + col] = rows[row * 4 + col];
                }
            }
            return new FrameTransform(r, rows[3], rows[7], rows[11]);
        }

        /// <summary>Writes the protocol's 3x4 form as three rows of four.</summary>
        public double[][] ToRows() => new[]
        {
            new[] { Rotation[0], Rotation[1], Rotation[2], X },
            new[] { Rotation[3], Rotation[4], Rotation[5], Y },
            new[] { Rotation[6], Rotation[7], Rotation[8], Z },
        };

        /// <summary>Returns a copy with the translation multiplied by <paramref name="factor"/> (metres to native unit is 1 / unit).</summary>
        public FrameTransform Scaled(double factor) => new FrameTransform(Rotation, X * factor, Y * factor, Z * factor);

        /// <summary>The z axis (third column), which is all a plane feature fixes besides its origin.</summary>
        public double[] ZAxis => new[] { Rotation[2], Rotation[5], Rotation[8] };

        /// <summary>
        /// Euler angles (x, y, z in radians) with R = Rx * Ry * Rz, the form
        /// CreateCoordinateSystemUsingNumericalValues takes (confirmed live on
        /// SOLIDWORKS 2025). The read-back comparison still guards a mismatch (R001).
        /// </summary>
        public double[] ToEulerXyz()
        {
            // frob:ticket 01M548FW58GD5Z00JGMAD835T5
            var r02 = Rotation[2];
            if (Math.Abs(r02) < 1 - 1e-12)
            {
                var b = Math.Asin(r02);
                var a = Math.Atan2(-Rotation[5], Rotation[8]);
                var c = Math.Atan2(-Rotation[1], Rotation[0]);
                return new[] { a, b, c };
            }
            // Gimbal lock: y is +-90deg, fold the z rotation into x.
            var pitch = r02 > 0 ? Math.PI / 2 : -Math.PI / 2;
            return new[] { Math.Atan2(Math.Sin(pitch) * Rotation[3], Rotation[4]), pitch, 0.0 };
        }

        /// <summary>
        /// True when every compared entry differs by at most 0.5 * 10^-precision
        /// (adapters.md 3.4); with <paramref name="originAndZOnly"/> only the
        /// translation and the z axis are compared, as for a plane feature.
        /// </summary>
        public bool Matches(FrameTransform other, int precision, bool originAndZOnly)
        {
            var tol = 0.5 * Math.Pow(10, -precision);
            if (Math.Abs(X - other.X) > tol || Math.Abs(Y - other.Y) > tol || Math.Abs(Z - other.Z) > tol)
            {
                return false;
            }
            for (var i = 0; i < 9; i++)
            {
                if (originAndZOnly && i % 3 != 2)
                {
                    continue;
                }
                if (Math.Abs(Rotation[i] - other.Rotation[i]) > tol)
                {
                    return false;
                }
            }
            return true;
        }

        /// <summary>Compact text for logs and R001-style messages.</summary>
        public override string ToString()
        {
            var inv = CultureInfo.InvariantCulture;
            return "[" + string.Join(", ", ToSwArray().AsSpanLike(12, inv)) + "]";
        }
    }

    internal static class ArrayText
    {
        /// <summary>Formats the first <paramref name="n"/> entries invariantly.</summary>
        public static IEnumerable<string> AsSpanLike(this double[] a, int n, CultureInfo inv)
        {
            for (var i = 0; i < n && i < a.Length; i++)
            {
                yield return a[i].ToString("0.######", inv);
            }
        }
    }
}

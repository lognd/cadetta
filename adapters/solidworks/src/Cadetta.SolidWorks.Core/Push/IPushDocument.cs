namespace Cadetta.SolidWorks.Push
{
    // frob:ticket 01M4SV2TR8ENTA87J1DDKCF816
    /// <summary>What the adapter knows about one native dimension before a push (values in the native unit).</summary>
    public sealed class DimensionInfo
    {
        /// <summary>The current value in the native unit.</summary>
        public double Value { get; set; }

        /// <summary>True for a driven (reference) dimension: R010 on push.</summary>
        public bool Driven { get; set; }

        /// <summary>True when the owning feature is suppressed: R010 on push.</summary>
        public bool Suppressed { get; set; }

        /// <summary>True when an equation or unit conflict prevents a write: R010 on push.</summary>
        public bool Conflict { get; set; }

        /// <summary>The global the dimension is linked to by an equation, or null.</summary>
        public string? LinkedGlobal { get; set; }

        /// <summary>The dimension's current tolerance.</summary>
        public ToleranceSpec Tolerance { get; set; } = ToleranceSpec.None;
    }

    /// <summary>One open document seen through the writes a push needs, in the native unit (solidworks.md 4); every write returns false when refused.</summary>
    public interface IPushDocument
    {
        /// <summary>Path of the document file.</summary>
        string Path { get; }

        /// <summary>True when the document cannot be written (read-only or checked out elsewhere): R009.</summary>
        bool ReadOnly { get; }

        /// <summary>True when the document holds edits the adapter did not make: R009.</summary>
        bool UserDirty { get; }

        /// <summary>True when the adapter changed the document in this session.</summary>
        bool AdapterModified { get; }

        /// <summary>The value of a global, or null when it does not exist.</summary>
        double? GetGlobal(string name);

        /// <summary>Creates or updates a global.</summary>
        bool SetGlobal(string name, double value);

        /// <summary>Removes a global (undo of a creation).</summary>
        bool DeleteGlobal(string name);

        /// <summary>Describes a dimension, or null when it does not resolve.</summary>
        DimensionInfo? GetDimension(string name);

        /// <summary>Sets a dimension's value (the dimension must not be linked).</summary>
        bool SetDimensionValue(string name, double value);

        /// <summary>Links a dimension to a global by the equation `"dim" = "global"`; a null global removes the link, keeping the value.</summary>
        bool SetDimensionLink(string name, string? global);

        /// <summary>Writes a dimension's tolerance; <see cref="ToleranceSpec.None"/> clears it.</summary>
        bool SetDimensionTolerance(string name, ToleranceSpec tolerance);

        /// <summary>The text of a custom property, or null when it does not exist.</summary>
        string? GetProperty(string name);

        /// <summary>Creates or updates a custom property.</summary>
        bool SetProperty(string name, string text);

        /// <summary>Removes a custom property (undo of a creation).</summary>
        bool DeleteProperty(string name);

        /// <summary>A coordinate system's transform as a row-major 3x4 (rotation then translation per row, native unit), or null when missing.</summary>
        double[]? GetFrame(string name);

        /// <summary>Sets a coordinate system's transform, creating it by numeric values when <paramref name="create"/> and it is missing.</summary>
        bool SetFrame(string name, double[] transform, bool create);

        /// <summary>Removes a coordinate system (undo of a creation).</summary>
        bool DeleteFrame(string name);

        /// <summary>Rebuilds the document; false when the rebuild reports errors.</summary>
        bool Rebuild();
    }
}

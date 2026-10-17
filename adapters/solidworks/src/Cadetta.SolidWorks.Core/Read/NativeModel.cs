using System.Collections.Generic;

namespace Cadetta.SolidWorks.Read
{
    /// <summary>
    /// A number as SOLIDWORKS holds it: a decimal literal, an optional unit, and
    /// a reason when it cannot be extracted faithfully (adapters.md 4 rule 4).
    /// frob:ticket 01M4SV2TKZD26CK9FFA1EJXSV4
    /// </summary>
    public sealed class NativeQuantity
    {
        /// <summary>Decimal literal in <see cref="Unit"/> (or the native unit when that is null).</summary>
        public string Text { get; set; } = "0";

        /// <summary>Explicit unit (mm, cm, m, in, deg, rad, or empty for a pure number); null means the native unit is assumed.</summary>
        public string? Unit { get; set; }

        /// <summary>Why the value is not faithful (driven dimension, unparsable expression); null when it is.</summary>
        public string? Reason { get; set; }

        /// <summary>Creates a quantity from a literal, with an optional unit and reason.</summary>
        public static NativeQuantity Of(string text, string? unit = null, string? reason = null) =>
            new NativeQuantity { Text = text, Unit = unit, Reason = reason };
    }

    /// <summary>An equation-manager global variable, with its equation text when it is computed.</summary>
    public sealed class NativeGlobal
    {
        /// <summary>The global's name.</summary>
        public string Name { get; set; } = "";

        /// <summary>Its evaluated value.</summary>
        public NativeQuantity Value { get; set; } = new NativeQuantity();

        /// <summary>The equation right-hand side (`"B" * 2`) when it is not a plain literal.</summary>
        public string? Expression { get; set; }
    }

    /// <summary>A dimension such as D1@Boss-Extrude1, with its equation, tolerance and driven state.</summary>
    public sealed class NativeDimension
    {
        /// <summary>The dimension locator, `D1@Feature` (the file is the document).</summary>
        public string Locator { get; set; } = "";

        /// <summary>Its value.</summary>
        public NativeQuantity Value { get; set; } = new NativeQuantity();

        /// <summary>The equation that drives it, when one does.</summary>
        public string? Expression { get; set; }

        /// <summary>The tolerance on the dimension, or null when untoleranced.</summary>
        public Protocol.DimTolerance? Tolerance { get; set; }
    }

    /// <summary>A custom property, file level or per configuration.</summary>
    public sealed class NativeProperty
    {
        /// <summary>The property name.</summary>
        public string Name { get; set; } = "";

        /// <summary>The configuration it belongs to; null for the file level.</summary>
        public string? Config { get; set; }

        /// <summary>The evaluated property value as text.</summary>
        public string Value { get; set; } = "";

        /// <summary>The unresolved expression (`"SW-Mass"`) when it differs from the value.</summary>
        public string? Expression { get; set; }
    }

    /// <summary>A coordinate system with its 3x4 transform in the native unit.</summary>
    public sealed class NativeFrame
    {
        /// <summary>The coordinate system's name.</summary>
        public string Name { get; set; } = "";

        /// <summary>Twelve entries, row-major 3x4: rotation columns then translation.</summary>
        public List<NativeQuantity> Transform { get; set; } = new List<NativeQuantity>();
    }

    /// <summary>A component instance in an assembly with its 3x4 transform and reference.</summary>
    public sealed class NativeInstance
    {
        /// <summary>The instance name (`bracket-1`).</summary>
        public string Component { get; set; } = "";

        /// <summary>The component's document path.</summary>
        public string Document { get; set; } = "";

        /// <summary>The component reference property, when one is set.</summary>
        public string? Reference { get; set; }

        /// <summary>Twelve entries, row-major 3x4.</summary>
        public List<NativeQuantity> Transform { get; set; } = new List<NativeQuantity>();
    }

    /// <summary>Everything `read` extracts from one SOLIDWORKS document.</summary>
    public sealed class NativeDocument
    {
        /// <summary>The document path as given to read.</summary>
        public string Path { get; set; } = "";

        /// <summary>Equation-manager globals.</summary>
        public List<NativeGlobal> Globals { get; } = new List<NativeGlobal>();

        /// <summary>Dimensions.</summary>
        public List<NativeDimension> Dimensions { get; } = new List<NativeDimension>();

        /// <summary>Custom properties.</summary>
        public List<NativeProperty> Properties { get; } = new List<NativeProperty>();

        /// <summary>Coordinate systems.</summary>
        public List<NativeFrame> Frames { get; } = new List<NativeFrame>();

        /// <summary>Component instances.</summary>
        public List<NativeInstance> Instances { get; } = new List<NativeInstance>();
    }

    /// <summary>Opens a SOLIDWORKS document and extracts its native model; the COM edge of `read`.</summary>
    public interface INativeDocumentReader
    {
        /// <summary>Extracts <paramref name="path"/>; throws when the file cannot be opened.</summary>
        NativeDocument Read(string path);
    }

    /// <summary>In-memory reader for tests: returns documents registered by path.</summary>
    public sealed class FakeNativeDocumentReader : INativeDocumentReader
    {
        private readonly Dictionary<string, NativeDocument> _docs = new Dictionary<string, NativeDocument>();

        /// <summary>Number of times <see cref="Read"/> was called.</summary>
        public int Reads { get; private set; }

        /// <summary>Registers <paramref name="doc"/> under its Path.</summary>
        public FakeNativeDocumentReader With(NativeDocument doc)
        {
            _docs[doc.Path] = doc;
            return this;
        }

        /// <summary>Returns the registered document or throws when unknown.</summary>
        public NativeDocument Read(string path)
        {
            Reads++;
            if (!_docs.TryGetValue(path, out var doc))
            {
                throw new System.IO.FileNotFoundException("no such document", path);
            }
            return doc;
        }
    }
}

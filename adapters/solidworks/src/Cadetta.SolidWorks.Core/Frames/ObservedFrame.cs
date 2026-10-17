using System.Text.Json.Serialization;

namespace Cadetta.SolidWorks.Frames
{
    // frob:ticket 01M4SWMDXKYAFXR9GEKVTE7XA4
    /// <summary>One entry of the observed IR's frames[]: a native frame and its transform (ir.md 1).</summary>
    public sealed class ObservedFrame
    {
        /// <summary>The frame locator (the feature name).</summary>
        [JsonPropertyName("name")]
        public string Name { get; set; } = "";

        /// <summary>The document holding the frame.</summary>
        [JsonPropertyName("document")]
        public string Document { get; set; } = "";

        /// <summary>Three rows of four numbers: rotation columns then translation in the native unit.</summary>
        [JsonPropertyName("transform")]
        public double[][] Transform { get; set; } = new double[0][];

        /// <summary>True when the locator named a plane feature, so only origin and z are meaningful.</summary>
        [JsonIgnore]
        public bool PlaneOnly { get; set; }
    }

    // frob:ticket 01M4SWMDXKYAFXR9GEKVTE7XA4
    /// <summary>One entry of read's sketches[]: where a bound sketch dimension sits, for R007 (adapters.md 3.4).</summary>
    public sealed class ObservedSketch
    {
        /// <summary>The document holding the sketch.</summary>
        [JsonPropertyName("file")]
        public string File { get; set; } = "";

        /// <summary>The dimension locator.</summary>
        [JsonPropertyName("locator")]
        public string Locator { get; set; } = "";

        /// <summary>True when the sketch lies on its datum's plane.</summary>
        [JsonPropertyName("on_datum_plane")]
        public bool OnDatumPlane { get; set; }

        /// <summary>What the dimension is measured from; `sketch origin` is the only clean answer.</summary>
        [JsonPropertyName("measured_from")]
        public string MeasuredFrom { get; set; } = "";
    }
}

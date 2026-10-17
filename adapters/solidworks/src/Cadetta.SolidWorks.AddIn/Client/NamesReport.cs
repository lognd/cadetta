// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Cadetta.SolidWorks.AddIn.Client
{
    /// <summary>One bindable path as `cadetta names --format json` reports it (docs/schemas/names.json).</summary>
    public sealed class NameRow
    {
        /// <summary>Canonical path.</summary>
        [JsonPropertyName("path")] public string Path { get; set; } = "";

        /// <summary>param, datum or place.</summary>
        [JsonPropertyName("kind")] public string Kind { get; set; } = "";

        /// <summary>Emit value text, null when none is writable.</summary>
        [JsonPropertyName("emit")] public string? Emit { get; set; }

        /// <summary>Authored tolerance text, null when untoleranced.</summary>
        [JsonPropertyName("tolerance")] public string? Tolerance { get; set; }

        /// <summary>Citation, if any.</summary>
        [JsonPropertyName("citation")] public string? Citation { get; set; }

        /// <summary>Doc comment, if any.</summary>
        [JsonPropertyName("doc")] public string? Doc { get; set; }

        /// <summary>The native name the adapter creates for the path.</summary>
        [JsonPropertyName("created_name")] public string CreatedName { get; set; } = "";

        /// <summary>explicit, implicit, unbound or unknown.</summary>
        [JsonPropertyName("state")] public string State { get; set; } = "";
    }

    /// <summary>One adapter's rows.</summary>
    public sealed class NamesAdapter
    {
        /// <summary>Adapter id.</summary>
        [JsonPropertyName("id")] public string Id { get; set; } = "";

        /// <summary>Rows sorted by canonical path.</summary>
        [JsonPropertyName("names")] public List<NameRow> Names { get; set; } = new List<NameRow>();
    }

    /// <summary>A names diagnostic.</summary>
    public sealed class NamesDiagnostic
    {
        /// <summary>Rule code.</summary>
        [JsonPropertyName("code")] public string Code { get; set; } = "";

        /// <summary>Message.</summary>
        [JsonPropertyName("message")] public string Message { get; set; } = "";
    }

    /// <summary>The `cadetta names --format json` document.</summary>
    public sealed class NamesReport
    {
        /// <summary>Per-adapter rows.</summary>
        [JsonPropertyName("adapters")] public List<NamesAdapter> Adapters { get; set; } = new List<NamesAdapter>();

        /// <summary>Diagnostics raised while listing.</summary>
        [JsonPropertyName("diagnostics")] public List<NamesDiagnostic> Diagnostics { get; set; } = new List<NamesDiagnostic>();

        /// <summary>Parses the CLI's JSON; null on malformed input.</summary>
        public static NamesReport? Parse(string json)
        {
            try
            {
                return JsonSerializer.Deserialize<NamesReport>(json);
            }
            catch (JsonException)
            {
                return null;
            }
        }
    }
}

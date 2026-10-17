// frob:ticket 01M54EDNCXH7QFEEGJQNAE1M1N
using System;
using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;
using Cadetta.SolidWorks.Commands;
using Cadetta.SolidWorks.Frames;

namespace Cadetta.SolidWorks.AddIn.Client
{
    /// <summary>One composed frame as `cadetta frames --format json` reports it (docs/schemas/frames.json).</summary>
    public sealed class FrameRow
    {
        /// <summary>`datum` or `instance`.</summary>
        [JsonPropertyName("kind")] public string Kind { get; set; } = "";

        /// <summary>Canonical path of the datum or placement.</summary>
        [JsonPropertyName("path")] public string Path { get; set; } = "";

        /// <summary>Three rows of four numbers (rotation then translation), or null when not composable.</summary>
        [JsonPropertyName("transform")] public List<List<double>>? Transform { get; set; }

        /// <summary>The transform as a <see cref="FrameTransform"/>; null when absent or not 3x4.</summary>
        public FrameTransform? Pose()
        {
            if (Transform == null || Transform.Count != 3)
            {
                return null;
            }
            var flat = new List<double>();
            foreach (var row in Transform)
            {
                if (row.Count != 4)
                {
                    return null;
                }
                flat.AddRange(row);
            }
            return FrameTransform.FromRows(flat);
        }
    }

    /// <summary>The `cadetta frames --format json` document (`cadetta.frames/1`).</summary>
    public sealed class FramesReport
    {
        /// <summary>The schema id this client reads.</summary>
        public const string SchemaId = "cadetta.frames/1";

        /// <summary>Schema id.</summary>
        [JsonPropertyName("schema")] public string Schema { get; set; } = "";

        /// <summary>Unit of the translations.</summary>
        [JsonPropertyName("native_unit")] public string NativeUnit { get; set; } = "";

        /// <summary>Decimal places of the transforms.</summary>
        [JsonPropertyName("precision")] public int Precision { get; set; }

        /// <summary>Datum and instance frames sorted by kind and path.</summary>
        [JsonPropertyName("frames")] public List<FrameRow> Frames { get; set; } = new List<FrameRow>();

        /// <summary>Parses the CLI's JSON; null on malformed input or another schema.</summary>
        public static FramesReport? Parse(string json)
        {
            try
            {
                var report = JsonSerializer.Deserialize<FramesReport>(json);
                return report != null && report.Schema == SchemaId ? report : null;
            }
            catch (JsonException)
            {
                return null;
            }
        }

        /// <summary>The composable instance frames under array <paramref name="prefix"/> (`pkg::Mount/feet`), named by their paths.</summary>
        public List<FrameCandidate> ArrayCandidates(string prefix)
        {
            var list = new List<FrameCandidate>();
            foreach (var f in Frames)
            {
                if (f.Kind != "instance" || ArrayAssign.ArrayPrefix(f.Path) != prefix)
                {
                    continue;
                }
                var pose = f.Pose();
                if (pose != null)
                {
                    list.Add(new FrameCandidate(f.Path, pose));
                }
            }
            return list;
        }
    }
}

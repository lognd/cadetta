// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
using System;
using System.Collections.Generic;
using System.IO;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.AddIn.Client
{
    /// <summary>A configured adapter: its name and the id the CLI addresses it by.</summary>
    public sealed class AdapterRef
    {
        /// <summary>Creates an adapter reference.</summary>
        public AdapterRef(string name, string id)
        {
            Name = name;
            Id = id;
        }

        /// <summary>The adapter's `name`.</summary>
        public string Name { get; }

        /// <summary>The adapter's `id` (defaults to the name).</summary>
        public string Id { get; }
    }

    /// <summary>A found project: its root folder, manifest and the solidworks adapters it configures.</summary>
    public sealed class ProjectInfo
    {
        /// <summary>Creates a project description.</summary>
        public ProjectInfo(string root, string manifest, IReadOnlyList<AdapterRef> adapters)
        {
            Root = root;
            Manifest = manifest;
            Adapters = adapters;
        }

        /// <summary>Folder holding cadetta.toml.</summary>
        public string Root { get; }

        /// <summary>Full path of cadetta.toml.</summary>
        public string Manifest { get; }

        /// <summary>Every `[[adapter]]` with name = "solidworks", in file order.</summary>
        public IReadOnlyList<AdapterRef> Adapters { get; }

        /// <summary>The adapter the pane uses: the one with <paramref name="chosenId"/>, else the first.</summary>
        public AdapterRef? Pick(string? chosenId)
        {
            if (chosenId != null)
            {
                foreach (var a in Adapters)
                {
                    if (a.Id == chosenId)
                    {
                        return a;
                    }
                }
            }
            return Adapters.Count == 0 ? null : Adapters[0];
        }
    }

    /// <summary>Finds the nearest cadetta.toml above a document and reads its solidworks adapters (solidworks.md 3).</summary>
    public static class ProjectDiscovery
    {
        /// <summary>Manifest file name.</summary>
        public const string ManifestName = "cadetta.toml";

        /// <summary>The adapter name this add-in drives.</summary>
        public const string AdapterName = "solidworks";

        /// <summary>Walks up from the folder of <paramref name="documentPath"/> to the nearest cadetta.toml; null when none.</summary>
        public static ProjectInfo? Find(string documentPath, Log log)
        {
            string? dir;
            try
            {
                dir = Path.GetDirectoryName(Path.GetFullPath(documentPath));
            }
            catch (ArgumentException)
            {
                log.Warn("project discovery: bad document path " + documentPath);
                return null;
            }
            while (!string.IsNullOrEmpty(dir))
            {
                var manifest = Path.Combine(dir, ManifestName);
                if (File.Exists(manifest))
                {
                    var adapters = ParseAdapters(File.ReadAllText(manifest));
                    log.Info("project discovery: " + manifest + " with " + adapters.Count + " solidworks adapter(s)");
                    return new ProjectInfo(dir!, manifest, adapters);
                }
                dir = Path.GetDirectoryName(dir);
            }
            log.Info("project discovery: no " + ManifestName + " above " + documentPath);
            return null;
        }

        /// <summary>Reads the `[[adapter]]` tables with name = "solidworks" from manifest text (line scan: the add-in never edits or fully parses TOML).</summary>
        public static IReadOnlyList<AdapterRef> ParseAdapters(string toml)
        {
            var found = new List<AdapterRef>();
            string? name = null;
            string? id = null;
            var inAdapter = false;
            void Flush()
            {
                if (inAdapter && name == AdapterName)
                {
                    found.Add(new AdapterRef(name!, id ?? name!));
                }
                name = null;
                id = null;
            }
            foreach (var raw in toml.Split('\n'))
            {
                var line = StripComment(raw).Trim();
                if (line.StartsWith("[", StringComparison.Ordinal))
                {
                    Flush();
                    inAdapter = line == "[[adapter]]";
                    continue;
                }
                if (!inAdapter)
                {
                    continue;
                }
                var eq = line.IndexOf('=');
                if (eq < 0)
                {
                    continue;
                }
                var key = line.Substring(0, eq).Trim();
                var value = Unquote(line.Substring(eq + 1).Trim());
                if (key == "name")
                {
                    name = value;
                }
                else if (key == "id")
                {
                    id = value;
                }
            }
            Flush();
            return found;
        }

        private static string StripComment(string line)
        {
            var quoted = false;
            for (var i = 0; i < line.Length; i++)
            {
                if (line[i] == '"' || line[i] == '\'')
                {
                    quoted = !quoted;
                }
                else if (line[i] == '#' && !quoted)
                {
                    return line.Substring(0, i);
                }
            }
            return line;
        }

        private static string? Unquote(string v) =>
            v.Length >= 2 && (v[0] == '"' || v[0] == '\'') && v[v.Length - 1] == v[0] ? v.Substring(1, v.Length - 2) : null;
    }
}

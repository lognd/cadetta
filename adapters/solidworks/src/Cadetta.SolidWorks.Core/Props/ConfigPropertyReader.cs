using System.Collections.Generic;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Read;

namespace Cadetta.SolidWorks.Props
{
    /// <summary>The configurations of a document and the properties read from the file level and the active configuration.</summary>
    public sealed class ConfigSnapshot
    {
        /// <summary>The active configuration; null when the document has none.</summary>
        public string? Active { get; set; }

        /// <summary>All configuration names, in native order.</summary>
        public List<string> Configurations { get; } = new List<string>();

        /// <summary>File-level properties then the active configuration's, each carrying its Config.</summary>
        public List<NativeProperty> Properties { get; } = new List<NativeProperty>();
    }

    /// <summary>
    /// Reads the active configuration and its custom properties (adapters.md 7.3:
    /// only the active configuration is imported; the others are only named).
    /// frob:ticket 01M4SWME29Y61DC7AKZD3HBZSH
    /// </summary>
    public static class ConfigPropertyReader
    {
        /// <summary>Collects file-level and active-configuration properties; a configuration property shadows no file-level one, both are kept.</summary>
        public static ConfigSnapshot Collect(IPropertySource source, Log log)
        {
            var snap = new ConfigSnapshot { Active = source.ActiveConfiguration };
            snap.Configurations.AddRange(source.Configurations);
            snap.Properties.AddRange(source.Read(null));
            if (snap.Active != null)
            {
                snap.Properties.AddRange(source.Read(snap.Active));
            }
            var skipped = snap.Configurations.Count - (snap.Active != null && snap.Configurations.Contains(snap.Active) ? 1 : 0);
            log.Info("configurations: active=" + (snap.Active ?? "(none)") + ", " + skipped + " not imported, "
                + snap.Properties.Count + " properties");
            return snap;
        }

        /// <summary>Copies <paramref name="snap"/> into <paramref name="doc"/>.</summary>
        public static void Apply(ConfigSnapshot snap, NativeDocument doc)
        {
            doc.ActiveConfig = snap.Active;
            doc.Configurations.AddRange(snap.Configurations);
            doc.Properties.AddRange(snap.Properties);
        }
    }
}

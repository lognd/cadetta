using System;
using System.Collections.Generic;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Props;
using Cadetta.SolidWorks.Read;
using SolidWorks.Interop.sldworks;

namespace Cadetta.SolidWorks.Com.Props
{
    /// <summary>
    /// IPropertySource over a SOLIDWORKS ModelDoc2: configuration names, the
    /// active configuration and CustomPropertyManager sets.
    /// frob:ticket 01M4SWME29Y61DC7AKZD3HBZSH
    /// </summary>
    public sealed class SwComPropertySource : IPropertySource
    {
        private readonly ModelDoc2 _model;
        private readonly Log _log;

        /// <summary>Wraps <paramref name="model"/>.</summary>
        public SwComPropertySource(ModelDoc2 model, Log log)
        {
            _model = model;
            _log = log;
        }

        /// <summary>The active configuration's name, null when the document has no configuration manager.</summary>
        public string? ActiveConfiguration => _model.ConfigurationManager?.ActiveConfiguration?.Name;

        /// <summary>All configuration names.</summary>
        public IReadOnlyList<string> Configurations => (string[]?)_model.GetConfigurationNames() ?? new string[0];

        /// <summary>Reads one property set; the evaluated value is the value, the raw text the expression when it differs.</summary>
        public IReadOnlyList<NativeProperty> Read(string? config)
        {
            var list = new List<NativeProperty>();
            var mgr = _model.Extension.CustomPropertyManager[config ?? ""];
            foreach (var name in (string[]?)mgr.GetNames() ?? new string[0])
            {
                string val, resolved;
                bool was;
                mgr.Get5(name, false, out val, out resolved, out was);
                list.Add(new NativeProperty
                {
                    Name = name,
                    Config = config,
                    Value = resolved ?? val ?? "",
                    Expression = val != null && val != resolved ? val : null,
                });
            }
            _log.Debug("properties of " + (config ?? "(file)") + ": " + list.Count);
            return list;
        }
    }
}

// frob:ticket 01M54EDNCXH7QFEEGJQNAE1M1N
using Cadetta.SolidWorks.Commands;
using Cadetta.SolidWorks.Logging;
using SolidWorks.Interop.sldworks;

namespace Cadetta.SolidWorks.AddIn
{
    /// <summary>Reads and writes IComponent2.ComponentReference of the active assembly's components (compile-only here; not exercised in SOLIDWORKS).</summary>
    public sealed class SwComComponentReferences : IComponentReferences
    {
        private readonly ISldWorks _sw;
        private readonly Log _log;

        /// <summary>Creates the accessor over the active document of <paramref name="sw"/>.</summary>
        public SwComComponentReferences(ISldWorks sw, Log log)
        {
            _sw = sw;
            _log = log;
        }

        private IComponent2? Find(string instance) => (_sw.ActiveDoc as IAssemblyDoc)?.GetComponentByName(instance) as IComponent2;

        /// <inheritdoc />
        public string? GetReference(string instance)
        {
            var value = Find(instance)?.ComponentReference;
            return string.IsNullOrEmpty(value) ? null : value;
        }

        /// <inheritdoc />
        public bool SetReference(string instance, string reference)
        {
            var comp = Find(instance);
            if (comp == null)
            {
                _log.Warn("component " + instance + " not found in the active assembly");
                return false;
            }
            comp.ComponentReference = reference;
            var ok = comp.ComponentReference == reference;
            _log.Info("component " + instance + " reference '" + reference + "' " + (ok ? "set" : "NOT kept"));
            return ok;
        }
    }
}

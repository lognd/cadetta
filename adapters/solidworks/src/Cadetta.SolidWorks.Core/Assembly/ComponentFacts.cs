using System.Collections.Generic;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Read;

namespace Cadetta.SolidWorks.Assembly
{
    /// <summary>
    /// Builds the `components` fact list of `read`: the file each component
    /// instance of an assembly actually holds, which the host compares with
    /// the slot primary's file for R011 (adapters.md 3.3, 5).
    /// frob:ticket 01M4SWME04D18722NZS7TE82RN
    /// </summary>
    public static class ComponentFacts
    {
        /// <summary>One fact per instance of <paramref name="assembly"/>, keyed by the instance locator (the component name).</summary>
        public static List<Protocol.ObservedComponent> From(NativeDocument assembly, Log log)
        {
            var file = ComponentNaming.FileOf(assembly.Path);
            var facts = new List<Protocol.ObservedComponent>();
            foreach (var i in assembly.Instances)
            {
                var instance = ComponentNaming.Instance(i.Component) ?? i.Component;
                var held = ComponentNaming.FileOf(i.Document);
                if (held.Length == 0)
                {
                    log.Warn("component " + instance + " of " + file + " has no document path; no fact");
                    continue;
                }
                log.Debug("component " + instance + " of " + file + " holds " + held);
                facts.Add(new Protocol.ObservedComponent { File = file, Locator = instance, ComponentFile = held });
            }
            return facts;
        }
    }
}

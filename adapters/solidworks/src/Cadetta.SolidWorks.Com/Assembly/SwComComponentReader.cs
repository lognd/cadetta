using System;
using System.Collections.Generic;
using Cadetta.SolidWorks.Assembly;
using Cadetta.SolidWorks.Com.Read;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Read;
using SolidWorks.Interop.sldworks;

namespace Cadetta.SolidWorks.Com.Assembly
{
    /// <summary>
    /// Reads the dimensions of each loaded component of an assembly so their
    /// locators carry the `component` field (adapters.md 3.5).
    /// frob:ticket 01M4SWME04D18722NZS7TE82RN
    /// </summary>
    public static class SwComComponentReader
    {
        /// <summary>Adds each loaded, distinct component document's dimensions to <paramref name="doc"/>, tagged with its instance name.</summary>
        public static void ReadDimensions(SwComDocumentReader reader, AssemblyDoc asm, NativeDocument doc, Log log)
        {
            foreach (var o in (object[]?)asm.GetComponents(true) ?? new object[0])
            {
                var c = (Component2)o;
                var instance = ComponentNaming.Instance(c.Name2);
                var model = c.GetModelDoc2() as ModelDoc2;
                if (instance == null || model == null)
                {
                    log.Debug("component " + c.Name2 + " skipped (no instance suffix or not loaded)");
                    continue;
                }
                var before = doc.Dimensions.Count;
                reader.WalkDimensions(model, doc.Dimensions, instance, ComponentNaming.FileOf(c.GetPathName()));
                log.Debug("component " + instance + ": " + (doc.Dimensions.Count - before) + " dimensions");
            }
        }
    }
}

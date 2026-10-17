using System.Collections.Generic;
using Cadetta.SolidWorks.Regen;
using SolidWorks.Interop.sldworks;

namespace Cadetta.SolidWorks.Com.Regen
{
    // frob:ticket 01M559F58RT4NP6T96P610BFDE
    /// <summary>Walks the feature tree after a rebuild and names each feature SOLIDWORKS flags with an error.</summary>
    internal static class SwComFeatureErrors
    {
        /// <summary>The errored features of <paramref name="model"/>; a failed rebuild naming none yields one nameless entry.</summary>
        public static List<RebuildError> Collect(IModelDoc2 model, bool rebuildOk)
        {
            var errors = new List<RebuildError>();
            for (var f = model.FirstFeature() as IFeature; f != null; f = f.GetNextFeature() as IFeature)
            {
                bool warn;
                var code = f.GetErrorCode2(out warn);
                if (code != 0 && !warn)
                {
                    errors.Add(new RebuildError { Feature = f.Name, Code = code, Message = "feature error code " + code });
                }
            }
            if (!rebuildOk && errors.Count == 0)
            {
                errors.Add(new RebuildError { Message = "ForceRebuild3 reported failure" });
            }
            return errors;
        }
    }
}

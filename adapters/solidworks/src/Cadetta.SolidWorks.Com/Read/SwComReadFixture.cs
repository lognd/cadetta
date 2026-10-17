using System;
using System.IO;
using Cadetta.SolidWorks.Logging;
using SolidWorks.Interop.sldworks;
using SolidWorks.Interop.swconst;

namespace Cadetta.SolidWorks.Com.Read
{
    /// <summary>
    /// Authors a part (symmetric tolerance on a dimension) and an assembly (component with a reference and
    /// transform) from an existing part, so the read path can be verified on a real SOLIDWORKS.
    /// frob:ticket 01M4SV2TKZD26CK9FFA1EJXSV4
    /// </summary>
    public sealed class SwComReadFixture
    {
        private readonly ISldWorks _sw;
        private readonly Log _log;

        /// <summary>Creates the fixture author over a running application.</summary>
        public SwComReadFixture(ISldWorks sw, Log log)
        {
            _sw = sw;
            _log = log;
        }

        /// <summary>Writes Fix.SLDPRT and FixAsm.SLDASM into the directory of the source part and returns the assembly path.</summary>
        public string Build(string sourcePart)
        {
            var dir = Path.GetDirectoryName(sourcePart)!;
            var part = Path.Combine(dir, "Fix.SLDPRT");
            var asmPath = Path.Combine(dir, "FixAsm.SLDASM");
            int err = 0, warn = 0;
            var doc = _sw.OpenDoc6(sourcePart, (int)swDocumentTypes_e.swDocPART, 0, "", ref err, ref warn);
            var dim = (Dimension)doc.Parameter("D1@Boss-Extrude1");
            var tol = dim.Tolerance;
            tol.Type = (int)swTolType_e.swTolSYMMETRIC;
            tol.SetValues(0.0002, 0.0002);
            _log.Info("fixture tolerance type=" + tol.Type + " max=" + tol.GetMaxValue() + " min=" + tol.GetMinValue());
            doc.Extension.SaveAs3(part, 0, 1, null, null, ref err, ref warn);
            _log.Info("fixture part saved " + part + " exists=" + File.Exists(part) + " err=" + err);

            var template = _sw.GetUserPreferenceStringValue((int)swUserPreferenceStringValue_e.swDefaultTemplateAssembly);
            _log.Info("fixture assembly template '" + template + "'");
            var asm = (ModelDoc2?)_sw.NewDocument(template, 0, 0, 0)
                ?? throw new InvalidOperationException("NewDocument returned null for '" + template + "'");
            var comp = ((AssemblyDoc)asm).AddComponent5(part, 0, "", false, "", 0.01, 0.02, 0.03)
                ?? throw new InvalidOperationException("AddComponent5 returned null for " + part);
            comp.ComponentReference = "REF_A";
            asm.ForceRebuild3(false);
            asm.Extension.SaveAs3(asmPath, 0, 1, null, null, ref err, ref warn);
            _log.Info("fixture assembly saved " + asmPath + " err=" + err);
            _sw.CloseDoc(asm.GetTitle());
            _sw.CloseDoc(((ModelDoc2)doc).GetTitle());
            return asmPath;
        }
    }
}

using System;
using System.Runtime.InteropServices;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Session;
using SolidWorks.Interop.sldworks;

namespace Cadetta.SolidWorks.Com
{
    /// <summary>ISwHost over COM: attaches via the running object table, else launches hidden.</summary>
    public sealed class SwComHost : ISwHost<ISldWorks>
    {
        private const string ProgId = "SldWorks.Application";
        private readonly Log _log;

        /// <summary>Creates a host that logs to <paramref name="log"/>.</summary>
        public SwComHost(Log log)
        {
            _log = log;
        }

        /// <summary>Returns the running SOLIDWORKS, or null when none is registered.</summary>
        public ISldWorks? TryAttach()
        {
            try
            {
                return (ISldWorks)Marshal.GetActiveObject(ProgId);
            }
            catch (COMException e)
            {
                _log.Debug("no running SOLIDWORKS: " + e.Message);
                return null;
            }
        }

        /// <summary>Launches SOLIDWORKS and keeps its window hidden.</summary>
        public ISldWorks StartHidden()
        {
            var type = Type.GetTypeFromProgID(ProgId)
                ?? throw new InvalidOperationException("SOLIDWORKS is not installed (no " + ProgId + " ProgID)");
            var sw = (ISldWorks)Activator.CreateInstance(type)!;
            sw.Visible = false;
            _log.Info("started SOLIDWORKS hidden");
            return sw;
        }

        /// <summary>Exits the SOLIDWORKS instance.</summary>
        public void Quit(ISldWorks app)
        {
            app.ExitApp();
        }
    }
}

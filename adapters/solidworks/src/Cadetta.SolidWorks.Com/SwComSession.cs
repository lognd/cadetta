using System.Runtime.InteropServices;
using Cadetta.SolidWorks.Session;
using SolidWorks.Interop.sldworks;

namespace Cadetta.SolidWorks.Com
{
    /// <summary>ISwSession over the SOLIDWORKS COM API.</summary>
    public sealed class SwComSession : ISwSession
    {
        private readonly ISldWorks _sw;

        /// <summary>Wraps a SOLIDWORKS application object.</summary>
        public SwComSession(ISldWorks sw)
        {
            _sw = sw;
        }

        /// <summary>Attaches to the running SOLIDWORKS; starting one hidden is a later change.</summary>
        public static SwComSession Attach()
        {
            return new SwComSession((ISldWorks)Marshal.GetActiveObject("SldWorks.Application"));
        }

        /// <summary>The SOLIDWORKS revision number.</summary>
        public string ProductVersion() => _sw.RevisionNumber();
    }
}

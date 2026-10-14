using Cadetta.SolidWorks.Logging;
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

        /// <summary>The connection that owns the application lifetime, or null when built from a bare handle.</summary>
        public SwConnection<ISldWorks>? Connection { get; }

        private SwComSession(SwConnection<ISldWorks> connection)
            : this(connection.App)
        {
            Connection = connection;
        }

        /// <summary>Attaches to the running SOLIDWORKS, otherwise starts one hidden (adapters.md 2.2).</summary>
        public static SwComSession Attach(Log log)
        {
            return new SwComSession(SwConnection<ISldWorks>.Connect(new SwComHost(log), log));
        }

        /// <summary>The SOLIDWORKS revision number.</summary>
        public string ProductVersion() => _sw.RevisionNumber();
    }
}

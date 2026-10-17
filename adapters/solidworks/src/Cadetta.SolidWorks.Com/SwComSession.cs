using System;
using Cadetta.SolidWorks.Com.Probe;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Probe;
using Cadetta.SolidWorks.Session;
using SolidWorks.Interop.sldworks;

namespace Cadetta.SolidWorks.Com
{
    /// <summary>ISwSession over the SOLIDWORKS COM API.</summary>
    public sealed class SwComSession : ISwSession
    {
        private readonly ISldWorks _sw;

        /// <summary>Wraps a SOLIDWORKS application object; internal so callers need no interop reference.</summary>
        internal SwComSession(ISldWorks sw)
        {
            _sw = sw;
        }

        /// <summary>The connection that owns the application lifetime, or null when built from a bare handle.</summary>
        public IDisposable? Connection { get; }

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

        /// <summary>The API probe for this application (solidworks.md 4).</summary>
        public ISwApiProbe ApiProbe(Log log) => new SwComApiProbe(_sw, log);

        /// <summary>Reads a document into observed IR through the COM reader (frob:ticket 01M4SV2TKZD26CK9FFA1EJXSV4).</summary>
        public Cadetta.SolidWorks.Protocol.ReadResult ReadObserved(string file, Log log)
        {
            var svc = new Cadetta.SolidWorks.Read.ReadService(new Read.SwComDocumentReader(_sw, log), null, log);
            return svc.Read(file);
        }

        /// <summary>A push host over this application (frob:ticket 01M5496CQE8FPX6F7PD9Y6EMX0).</summary>
        public IPushHost PushHost(Log log) => new Session.SwComPushHost(_sw, log);

        /// <summary>A regen host over this application (frob:ticket 01M4SV2TVM50WJRZ42ZHXVD96N).</summary>
        public Cadetta.SolidWorks.Regen.IRegenHost RegenHost(Log log) => new Regen.SwComRegenHost(_sw, log);

        /// <summary>A cached read service over this application (frob:ticket 01M5496CQE8FPX6F7PD9Y6EMX0).</summary>
        public Cadetta.SolidWorks.Read.ReadService ReadService(Log log) =>
            new Cadetta.SolidWorks.Read.ReadService(new Read.SwComDocumentReader(_sw, log),
                new Cadetta.SolidWorks.Cache.ReadCache(new DiskFileReader(), log), log);

        /// <summary>Authors the read-verification fixture next to a source part (frob:ticket 01M4SV2TKZD26CK9FFA1EJXSV4).</summary>
        public string MakeReadFixture(string sourcePart, Log log) => new Read.SwComReadFixture(_sw, log).Build(sourcePart);

        /// <summary>The SOLIDWORKS revision number.</summary>
        public string ProductVersion() => _sw.RevisionNumber();
    }
}

using System.Collections.Generic;
using System.Text.Json;
using Cadetta.SolidWorks.Protocol;
using Cadetta.SolidWorks.Save;

namespace Cadetta.SolidWorks.Push
{
    // frob:ticket 01M4SV2TR8ENTA87J1DDKCF816
    /// <summary>Result of one document push: a status per locator, diagnostics, and the save step's outcome.</summary>
    public sealed class PushOutcome
    {
        /// <summary>One status per locator, in request order.</summary>
        public List<LocatorStatus> Statuses { get; } = new List<LocatorStatus>();

        /// <summary>`cadetta.diag/1` records for the push.</summary>
        public List<JsonElement> Diagnostics { get; } = new List<JsonElement>();

        /// <summary>True when a rebuild failed after the push (R004), restored or not.</summary>
        public bool RebuildFailed { get; set; }

        /// <summary>True when any locator ended failed or rejected.</summary>
        public bool AnyFailed { get; set; }

        /// <summary>The save step's outcome; null when no save was requested.</summary>
        public SaveOutcome? Save { get; set; }

        /// <summary>The wire form of the push result.</summary>
        public PushResult ToResult()
        {
            var r = new PushResult();
            r.Statuses.AddRange(Statuses);
            r.Diagnostics.AddRange(Diagnostics);
            return r;
        }
    }
}

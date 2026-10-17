namespace Cadetta.SolidWorks.Probe
{
    // frob:ticket ~Z9SWC8W
    /// <summary>The outcome of exercising one SOLIDWORKS API (solidworks.md 4).</summary>
    public sealed class ApiProbeResult
    {
        /// <summary>Records the outcome of <paramref name="api"/> called as <paramref name="call"/>.</summary>
        public ApiProbeResult(string api, string call, bool ok, string detail)
        {
            Api = api;
            Call = call;
            Ok = ok;
            Detail = detail;
        }

        /// <summary>The API area from solidworks.md 4, such as "equation manager".</summary>
        public string Api { get; }

        /// <summary>The call used, as written in the C# interop.</summary>
        public string Call { get; }

        /// <summary>True when the call worked and returned what the spec needs.</summary>
        public bool Ok { get; }

        /// <summary>What was observed: the returned value on success, the failure otherwise.</summary>
        public string Detail { get; }
    }
}

using Cadetta.SolidWorks.Save;

namespace Cadetta.SolidWorks.Push
{
    // frob:ticket 01M4SV2TR8ENTA87J1DDKCF816
    /// <summary>The adapter settings a push honours (adapters.md 2.2, 3.4, 3.6).</summary>
    public sealed class PushOptions
    {
        /// <summary>Fractional decimal digits of the native number format (`initialize` precision).</summary>
        public int Precision { get; set; } = 6;

        /// <summary>`datums = "create"`: a missing coordinate system is created from numeric values.</summary>
        public bool CreateDatums { get; set; }

        /// <summary>Prepended to created global names (adapters.md 3.1).</summary>
        public string NamePrefix { get; set; } = "";

        /// <summary>When modified documents are written after the push.</summary>
        public SaveMode Save { get; set; } = SaveMode.AfterSync;
    }
}

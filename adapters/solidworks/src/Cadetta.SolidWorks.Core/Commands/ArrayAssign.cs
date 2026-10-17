using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text;
using Cadetta.SolidWorks.Frames;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.Commands
{
    // frob:ticket 01M54EDNCXH7QFEEGJQNAE1M1N
    /// <summary>One selected component instance and its pose in the assembly frame (metres).</summary>
    public sealed class SelectedInstance
    {
        /// <summary>Creates a selected instance.</summary>
        public SelectedInstance(string name, FrameTransform pose)
        {
            Name = name;
            Pose = pose;
        }

        /// <summary>The instance name such as `Foot-1`.</summary>
        public string Name { get; }

        /// <summary>The instance transform in metres.</summary>
        public FrameTransform Pose { get; }
    }

    // frob:ticket 01M54EDNCXH7QFEEGJQNAE1M1N
    /// <summary>Reads and writes the reference property of component instances (the SOLIDWORKS ComponentReference).</summary>
    public interface IComponentReferences
    {
        /// <summary>The reference property of <paramref name="instance"/>, or null when empty.</summary>
        string? GetReference(string instance);

        /// <summary>Sets the reference property; false when SOLIDWORKS refuses.</summary>
        bool SetReference(string instance, string reference);
    }

    // frob:ticket 01M54EDNCXH7QFEEGJQNAE1M1N
    /// <summary>Assign array: gives each selected instance the reference of the unique nearest composed instance frame, all or nothing (solidworks.md 2.2).</summary>
    public static class ArrayAssign
    {
        /// <summary>Largest origin distance (metres) between an instance and its matched frame.</summary>
        public const double DefaultMaxDistance = 0.001;

        /// <summary>The array a placement path belongs to: `pkg::Mount/feet[2].place` gives `pkg::Mount/feet`; null when the path is not an indexed placement.</summary>
        public static string? ArrayPrefix(string path)
        {
            var placeAt = path.IndexOf(".place", StringComparison.Ordinal);
            var p = placeAt > 0 ? path.Substring(0, placeAt) : path;
            var open = p.LastIndexOf('[');
            if (open <= 0 || !p.EndsWith("]", StringComparison.Ordinal))
            {
                return null;
            }
            var digits = p.Substring(open + 1, p.Length - open - 2);
            if (digits.Length == 0 || digits.Trim('0', '1', '2', '3', '4', '5', '6', '7', '8', '9').Length != 0)
            {
                return null;
            }
            return p.Substring(0, open);
        }

        /// <summary>The reference property text of an instance frame path: `pkg::Mount/feet[2].place` gives `feet[2]` (the part's instance path relative to its assembly, adapters.md 3.5); null without a `/`.</summary>
        public static string? ReferenceOf(string instancePath)
        {
            var p = instancePath.EndsWith(".place", StringComparison.Ordinal) ? instancePath.Substring(0, instancePath.Length - 6) : instancePath;
            var slash = p.IndexOf('/');
            return slash < 0 || slash == p.Length - 1 ? null : p.Substring(slash + 1);
        }

        /// <summary>The matching tolerance for a frames <paramref name="precision"/>: half a unit in the last place, in metres.</summary>
        public static double TieEpsilon(int precision) => 0.5 * Math.Pow(10, -precision);

        /// <summary>
        /// Matches every selected instance to its nearest candidate (candidate names are instance frame
        /// paths) and sets the references. Refuses, writing nothing, when any match is missing, too far,
        /// tied or shared by two instances; undoes the writes when one fails.
        /// </summary>
        public static CommandOutcome Assign(IReadOnlyList<SelectedInstance> selected, IReadOnlyList<FrameCandidate> candidates, double maxDistance, double tieEpsilon, IComponentReferences refs, Log log)
        {
            if (selected.Count == 0)
            {
                log.Warn("assign array: nothing selected");
                return CommandOutcome.Refused("select the component instances of the array first");
            }
            var chosen = new List<string>();
            foreach (var s in selected)
            {
                var outcome = FrameMatcher.Nearest(candidates, s.Pose, maxDistance, tieEpsilon, log, out var frame);
                if (!outcome.IsOk)
                {
                    return CommandOutcome.Refused(s.Name + ": " + outcome.Message);
                }
                var reference = ReferenceOf(frame!.Name);
                if (reference == null)
                {
                    log.Error("assign array: frame path " + frame.Name + " has no assembly-relative part");
                    return CommandOutcome.Refused(s.Name + ": frame " + frame.Name + " has no reference path");
                }
                if (chosen.Contains(reference))
                {
                    log.Warn("assign array: " + reference + " matched twice");
                    return CommandOutcome.Refused(s.Name + ": " + reference + " is the nearest frame of another selected instance too");
                }
                chosen.Add(reference);
            }
            var previous = new List<string?>();
            for (var i = 0; i < selected.Count; i++)
            {
                previous.Add(refs.GetReference(selected[i].Name));
                if (!refs.SetReference(selected[i].Name, chosen[i]))
                {
                    log.Error("assign array: SOLIDWORKS refused the reference of " + selected[i].Name + "; undoing " + i + " write(s)");
                    for (var j = 0; j < i; j++)
                    {
                        refs.SetReference(selected[j].Name, previous[j] ?? "");
                    }
                    return CommandOutcome.Refused("could not set the reference of " + selected[i].Name + "; nothing was assigned");
                }
            }
            var text = new StringBuilder("assigned ");
            for (var i = 0; i < selected.Count; i++)
            {
                text.Append(i == 0 ? "" : ", ").Append(selected[i].Name).Append(" -> ").Append(chosen[i]);
            }
            log.Info("assign array: " + text.ToString(9, text.Length - 9));
            return CommandOutcome.Ok(text.ToString());
        }
    }
}

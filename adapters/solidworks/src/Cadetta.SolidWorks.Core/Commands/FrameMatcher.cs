using System;
using System.Collections.Generic;
using Cadetta.SolidWorks.Frames;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.Commands
{
    // frob:ticket 01M4VB7VXXHJ6TKHF6J2STC1FE
    /// <summary>A named candidate frame (a component instance or a datum) with its pose in the parent frame.</summary>
    public sealed class FrameCandidate
    {
        /// <summary>Creates a candidate.</summary>
        public FrameCandidate(string name, FrameTransform pose)
        {
            Name = name;
            Pose = pose;
        }

        /// <summary>Instance or frame name.</summary>
        public string Name { get; }

        /// <summary>Pose in the parent frame.</summary>
        public FrameTransform Pose { get; }
    }

    /// <summary>Picks the one candidate nearest a target pose, or refuses when none is unique (assign array).</summary>
    public static class FrameMatcher
    {
        /// <summary>Returns the nearest candidate by origin distance; refuses when empty, farther than <paramref name="maxDistance"/>, or tied within <paramref name="tieEpsilon"/>.</summary>
        public static CommandOutcome Nearest(IReadOnlyList<FrameCandidate> candidates, FrameTransform target, double maxDistance, double tieEpsilon, Log log, out FrameCandidate? chosen)
        {
            chosen = null;
            if (candidates.Count == 0)
            {
                return CommandOutcome.Refused("no candidate frames to match");
            }
            FrameCandidate? best = null;
            var bestD = double.MaxValue;
            var tied = false;
            foreach (var c in candidates)
            {
                var d = Distance(c.Pose, target);
                if (d < bestD - tieEpsilon)
                {
                    best = c;
                    bestD = d;
                    tied = false;
                }
                else if (Math.Abs(d - bestD) <= tieEpsilon)
                {
                    tied = true;
                }
            }
            if (bestD > maxDistance)
            {
                log.Warn("nearest frame " + best!.Name + " is " + bestD.ToString("0.######", System.Globalization.CultureInfo.InvariantCulture) + " away, beyond " + maxDistance);
                return CommandOutcome.Refused("nearest frame is farther than " + maxDistance + " from the selection");
            }
            if (tied)
            {
                log.Warn("ambiguous nearest frame at distance " + bestD);
                return CommandOutcome.Refused("more than one frame is equally near the selection; select a single instance");
            }
            chosen = best;
            log.Info("matched instance " + best!.Name);
            return CommandOutcome.Ok("matched " + best.Name);
        }

        /// <summary>Euclidean distance between two origins.</summary>
        public static double Distance(FrameTransform a, FrameTransform b)
        {
            var dx = a.X - b.X;
            var dy = a.Y - b.Y;
            var dz = a.Z - b.Z;
            return Math.Sqrt(dx * dx + dy * dy + dz * dz);
        }
    }
}

package cluster

import "time"

type HealthConfig struct {
	SuspectAfter time.Duration
	DeadAfter    time.Duration
}

func DefaultHealthConfig() HealthConfig {
	return HealthConfig{
		SuspectAfter: 30 * time.Second,
		DeadAfter:    90 * time.Second,
	}
}

func (r *Registry) EvaluateHealth(now time.Time, cfg HealthConfig) {
	r.mu.Lock()
	defer r.mu.Unlock()

	for id, info := range r.nodes {
		if info.LastSeen.IsZero() {
			info.State = NodeUnknown
			r.nodes[id] = info
			continue
		}

		age := now.Sub(info.LastSeen)

		switch {
		case age >= cfg.DeadAfter:
			info.State = NodeDead
		case age >= cfg.SuspectAfter:
			info.State = NodeSuspect
		default:
			info.State = NodeAlive
		}

		r.nodes[id] = info
	}
}

{{- define "openab.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "openab.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{- define "openab.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "openab.labels" -}}
helm.sh/chart: {{ include "openab.chart" .ctx }}
app.kubernetes.io/name: {{ include "openab.name" .ctx }}
app.kubernetes.io/instance: {{ .ctx.Release.Name }}
app.kubernetes.io/component: {{ .agent }}
{{- if .ctx.Chart.AppVersion }}
app.kubernetes.io/version: {{ .ctx.Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .ctx.Release.Service }}
{{- end }}

{{- define "openab.selectorLabels" -}}
app.kubernetes.io/name: {{ include "openab.name" .ctx }}
app.kubernetes.io/instance: {{ .ctx.Release.Name }}
app.kubernetes.io/component: {{ .agent }}
{{- end }}

{{/* Per-agent resource name: <fullname>-<agentKey> */}}
{{- define "openab.agentFullname" -}}
{{- printf "%s-%s" (include "openab.fullname" .ctx) .agent | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/* Resolve image: agent-level string override -> preset image -> global default (repository:tag). */}}
{{- define "openab.agentImage" -}}
{{- if and .cfg.image (kindIs "string" .cfg.image) (ne .cfg.image "") }}
{{- .cfg.image }}
{{- else }}
{{- $tag := default .ctx.Chart.AppVersion .ctx.Values.image.tag }}
{{- $preset := default "" .cfg.preset }}
{{- if eq $preset "" }}
{{- printf "%s:%s" .ctx.Values.image.repository $tag }}
{{- else if eq $preset "kiro" }}
{{- printf "%s:%s" .ctx.Values.image.repository $tag }}
{{- else if eq $preset "codex" }}
{{- printf "%s-codex:%s" .ctx.Values.image.repository $tag }}
{{- else if eq $preset "claude" }}
{{- printf "%s-claude:%s" .ctx.Values.image.repository $tag }}
{{- else if eq $preset "gemini" }}
{{- printf "%s-gemini:%s" .ctx.Values.image.repository $tag }}
{{- else if eq $preset "opencode" }}
{{- printf "%s-opencode:%s" .ctx.Values.image.repository $tag }}
{{- else }}
{{- fail (printf "unsupported agents.%s.preset %q" .agent $preset) }}
{{- end }}
{{- end }}
{{- end }}

{{/* Resolve command: preset wins when set; otherwise use explicit command. */}}
{{- define "openab.agentCommand" -}}
{{- $preset := default "" .cfg.preset }}
{{- if eq $preset "" }}
{{- required (printf "agents.%s.command is required when preset is empty" .agent) .cfg.command }}
{{- else if eq $preset "kiro" }}kiro-cli
{{- else if eq $preset "codex" }}codex-acp
{{- else if eq $preset "claude" }}claude-agent-acp
{{- else if eq $preset "gemini" }}gemini
{{- else if eq $preset "opencode" }}opencode
{{- else }}{{- fail (printf "unsupported agents.%s.preset %q" .agent $preset) }}
{{- end }}
{{- end }}

{{/* Resolve args: preset wins when set; otherwise use explicit args. */}}
{{- define "openab.agentArgs" -}}
{{- $preset := default "" .cfg.preset }}
{{- if eq $preset "" }}
{{- if .cfg.args }}{{ .cfg.args | toJson }}{{ else }}[]{{ end }}
{{- else if eq $preset "kiro" }}["acp","--trust-all-tools"]
{{- else if or (eq $preset "codex") (eq $preset "claude") }}[]
{{- else if eq $preset "gemini" }}["--acp"]
{{- else if eq $preset "opencode" }}["acp"]
{{- else }}{{- fail (printf "unsupported agents.%s.preset %q" .agent $preset) }}
{{- end }}
{{- end }}

{{/* Resolve working directory: preset wins when set; otherwise use explicit value. */}}
{{- define "openab.agentWorkingDir" -}}
{{- $preset := default "" .cfg.preset }}
{{- if eq $preset "" }}
{{- .cfg.workingDir | default "/home/agent" }}
{{- else if eq $preset "kiro" }}/home/agent
{{- else if or (eq $preset "codex") (eq $preset "claude") (eq $preset "gemini") (eq $preset "opencode") }}/home/node
{{- else }}{{- fail (printf "unsupported agents.%s.preset %q" .agent $preset) }}
{{- end }}
{{- end }}

{{/* Resolve imagePullPolicy: global default (per-agent image string has no pullPolicy). */}}
{{- define "openab.agentImagePullPolicy" -}}
{{- .ctx.Values.image.pullPolicy }}
{{- end }}

{{/* Agent enabled: default true unless explicitly set to false. */}}
{{- define "openab.agentEnabled" -}}
{{- if eq (.enabled | toString) "false" }}false{{ else }}true{{ end }}
{{- end }}

{{/* Persistence enabled: default true unless explicitly set to false. */}}
{{- define "openab.persistenceEnabled" -}}
{{- if and . .persistence (eq (.persistence.enabled | toString) "false") }}false{{ else }}true{{ end }}
{{- end }}

package com.phillipchin.webrtctunnel.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Divider
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

private val Success = Color(0xFF2E7D32)
private val Warning = Color(0xFFF59E0B)
private val Error = Color(0xFFD32F2F)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TunnelTopAppBar(title: String, navigationIcon: @Composable (() -> Unit)? = null) {
    TopAppBar(
        title = { Text(title, style = MaterialTheme.typography.titleSmall) },
        colors = TopAppBarDefaults.topAppBarColors(
            containerColor = Color(0xFF061A3D),
            titleContentColor = Color.White,
            navigationIconContentColor = Color.White,
        ),
        navigationIcon = { navigationIcon?.invoke() },
    )
}

@Composable
fun SectionHeader(title: String, subtitle: String? = null) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(title, style = MaterialTheme.typography.titleLarge)
        subtitle?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = Color(0xFF6B7280)) }
    }
}

@Composable
fun StatusCard(content: @Composable () -> Unit) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp), content = { content() })
    }
}

@Composable
fun NetworkStatusCard(content: @Composable () -> Unit) = StatusCard(content = content)

@Composable
fun ForwardSummaryRow(title: String, subtitle: String, status: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column {
            Text(title, style = MaterialTheme.typography.titleMedium)
            Text(subtitle, style = MaterialTheme.typography.bodySmall, color = Color(0xFF6B7280))
        }
        AssistChip(onClick = {}, label = { Text(status) })
    }
}

@Composable
fun EmptyStateCard(message: String) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Text(message, modifier = Modifier.padding(16.dp), style = MaterialTheme.typography.bodyMedium)
    }
}

@Composable
fun ErrorResolutionCard(summary: String, fix: String, details: String? = null, action: @Composable (() -> Unit)? = null) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = Color(0xFFFFF5F5)),
    ) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(summary, color = Error, style = MaterialTheme.typography.titleMedium)
            Text(fix, style = MaterialTheme.typography.bodyMedium)
            details?.takeIf { it.isNotBlank() }?.let {
                HorizontalDivider()
                Text(it, style = MaterialTheme.typography.bodySmall, color = Color(0xFF6B7280))
            }
            action?.invoke()
        }
    }
}

@Composable
fun WizardStepper(steps: List<String>, currentIndex: Int) {
    Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        steps.forEachIndexed { index, title ->
            val active = index == currentIndex
            val color = if (active) MaterialTheme.colorScheme.primary else Color(0xFFE5E7EB)
            Box(
                modifier = Modifier
                    .weight(1f)
                    .heightIn(min = 32.dp)
                    .background(color, RoundedCornerShape(10.dp))
                    .padding(horizontal = 8.dp, vertical = 6.dp),
            ) {
                Text(
                    "${index + 1}. ${title}",
                    color = if (active) Color.White else Color(0xFF6B7280),
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }
    }
}

@Composable
fun SettingsSection(title: String, content: @Composable () -> Unit) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(title, style = MaterialTheme.typography.titleMedium)
            content()
        }
    }
}

@Composable
fun DestructiveActionButton(text: String, onClick: () -> Unit) {
    OutlinedButton(
        onClick = onClick,
        modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp),
    ) {
        Text(text, color = Error)
    }
}

fun stateColorToken(state: String): Color = when {
    state.contains("connected", ignoreCase = true) || state.contains("listening", ignoreCase = true) -> Success
    state.contains("paused", ignoreCase = true) || state.contains("starting", ignoreCase = true) -> Warning
    state.contains("error", ignoreCase = true) || state.contains("invalid", ignoreCase = true) -> Error
    else -> Color(0xFF6B7280)
}

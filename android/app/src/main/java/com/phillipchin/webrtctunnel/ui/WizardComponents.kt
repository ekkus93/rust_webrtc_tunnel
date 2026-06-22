package com.phillipchin.webrtctunnel.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

@Composable
fun WizardStepper(
    steps: List<String>,
    currentIndex: Int,
) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            steps.forEachIndexed { index, _ ->
                Box(modifier = Modifier.weight(1f), contentAlignment = Alignment.Center) {
                    WizardStepIndicator(
                        stepNumber = index + 1,
                        active = index == currentIndex,
                        completed = index < currentIndex,
                        showConnector = index < steps.lastIndex,
                    )
                }
            }
        }
        Text(
            "Step ${currentIndex + 1} of ${steps.size}: ${steps[currentIndex]}",
            style = MaterialTheme.typography.titleSmall,
        )
    }
}

@Composable
private fun WizardStepIndicator(
    stepNumber: Int,
    active: Boolean,
    completed: Boolean,
    showConnector: Boolean,
) {
    val circleColor =
        when {
            active -> MaterialTheme.colorScheme.primary
            completed -> MaterialTheme.colorScheme.primaryContainer
            else -> Color(color = 0xFFE5E7EB)
        }
    Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Box(
            modifier =
                Modifier
                    .heightIn(min = 32.dp)
                    .background(circleColor, RoundedCornerShape(999.dp))
                    .padding(horizontal = 10.dp, vertical = 6.dp),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                "$stepNumber",
                color = if (active) Color.White else Color(color = 0xFF374151),
                style = MaterialTheme.typography.bodySmall,
            )
        }
        if (showConnector) {
            Box(
                modifier =
                    Modifier
                        .weight(1f)
                        .heightIn(min = 2.dp)
                        .padding(horizontal = 4.dp)
                        .background(
                            if (completed) {
                                MaterialTheme.colorScheme.primary
                            } else {
                                Color(color = 0xFFD1D5DB)
                            },
                        ),
            )
        }
    }
}

@Composable
fun SettingsSection(
    title: String,
    content: @Composable () -> Unit,
) {
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

const val METERED_WARNING_MESSAGE =
    "WebRTC Tunnel can use significant mobile data and may incur overage charges or " +
        "throttling. Enable cellular or metered use only if you understand and accept this risk."

@Composable
fun MeteredWarningDialog(
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Cellular / Metered Data Warning") },
        text = { Text(METERED_WARNING_MESSAGE) },
        confirmButton = { TextButton(onClick = onConfirm) { Text("I understand") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
fun ScrollableScreenSurface(
    padding: androidx.compose.foundation.layout.PaddingValues,
    content: @Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
        verticalArrangement = Arrangement.Top,
        content = content,
    )
}

@Composable
fun DestructiveActionButton(
    text: String,
    enabled: Boolean = true,
    onClick: () -> Unit,
) {
    AppOutlinedButton(
        onClick = onClick,
        enabled = enabled,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text(text, color = Error)
    }
}

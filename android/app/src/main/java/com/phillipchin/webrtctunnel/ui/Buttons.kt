package com.phillipchin.webrtctunnel.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

internal val AppButtonShape = RoundedCornerShape(10.dp)
private val AppButtonContentPadding = PaddingValues(horizontal = 20.dp, vertical = 14.dp)

/** Filled primary-action button with the app's refined corner radius and generous padding. */
@Composable
fun AppFilledButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    content: @Composable RowScope.() -> Unit,
) {
    Button(
        onClick = onClick,
        modifier = modifier,
        enabled = enabled,
        shape = AppButtonShape,
        contentPadding = AppButtonContentPadding,
        content = content,
    )
}

/**
 * Outlined secondary-action button — firmer 1.5 dp border with `onSurfaceVariant` ink
 * instead of the near-invisible default `outline` color.
 */
@Composable
fun AppOutlinedButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    content: @Composable RowScope.() -> Unit,
) {
    OutlinedButton(
        onClick = onClick,
        modifier = modifier,
        enabled = enabled,
        shape = AppButtonShape,
        border =
            BorderStroke(
                1.5.dp,
                MaterialTheme.colorScheme.onSurfaceVariant.copy(
                    alpha = if (enabled) 0.45f else 0.2f,
                ),
            ),
        contentPadding = AppButtonContentPadding,
        content = content,
    )
}

/** Text-only button for low-emphasis and dialog actions. */
@Composable
fun AppTextButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    content: @Composable RowScope.() -> Unit,
) {
    TextButton(
        onClick = onClick,
        modifier = modifier,
        enabled = enabled,
        shape = AppButtonShape,
        contentPadding = AppButtonContentPadding,
        content = content,
    )
}
